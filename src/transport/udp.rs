// Pitot - a customizable aviation information receiver
// Copyright (C) 2017  Datong Sun (dndx@idndx.com)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::collections::{HashSet, HashMap};
use std::net::{UdpSocket, Ipv4Addr, IpAddr};
use std::io::{self, ErrorKind, Read};
use std::fs::File;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use time::{Timespec, Tm, now_utc};
use nom::{IResult, be_u8, be_u32, be_u64};
use inotify::{Inotify, watch_mask};
use icmp::IcmpSocket;
use super::*;

const LEASE_FILE_PATH: &str = "/tmp/udhcpd.leases";
const WATCH_PATH: &str = "/tmp";
const GDL90_PORT: u16 = 4000;
const UDP_MAX_SIZE: usize = 1472; // maximum UDP payload size without fragmentation in Ethernet environment
const INACTIVE_BUFFER_SIZE: usize = 8192; // maximum number of messages to buffer and later reply back to sleeping clients
const PING_PACKET: [u8; 13] = [
    0x08, 0x00, 0x25, 0xc9,
    0xd9, 0x9d, // identifier magic
    0x00, 0x00, // sequence number
    'P' as u8, 'I' as u8, 'T' as u8, 'O' as u8, 'T' as u8,
];
const PING_FREQ: u32 = 1;
const DEAD_THRESHOLD: u64 = 10; // if no ping response has been received in this much seconds, consider the client as inactive
const IN_APP_THRESHOLD: u64 = 5; // if no "connection refused" has been received in this much seconds, consider the client as back to the App
const REPLAY_INTERVAL: u64 = 30; // at mist 1 replay can be delivered to a client in REPLAY_INTERVAL seconds

struct Client {
    udp_sock: UdpSocket,
    icmp_sock: IcmpSocket,
    active: bool,
    last_reply: Instant,
    in_app: bool,
    last_refused: Instant,
    last_replay: Instant,
}

pub struct UDP {
    clients: HashMap<Ipv4Addr, Client>,
    inotify: Inotify,
    queue: VecDeque<Payload>,
    inactive_buffer: VecDeque<Payload>,
    ping_counter: u32,
}

named_args!(parse_ip_from_lease_file(tm: Timespec, cap: usize)<HashSet<Ipv4Addr>>,
       do_parse!(
           written: be_u64 >>
           ips: fold_many0!(
               do_parse!(
                   expires: be_u32 >>
                   a: be_u8 >>
                   b: be_u8 >>
                   c: be_u8 >>
                   d: be_u8 >>
                   take!(6 + 20 + 2) >> // skip mac, hostname and pad
                   (a, b, c, d, expires)
               ), HashSet::with_capacity(cap), |mut acc: HashSet<Ipv4Addr>, info: (u8, u8, u8, u8, u32)| {
                      if tm.sec < (info.4 as u64 + written) as i64 {
                        acc.insert(Ipv4Addr::new(info.0, info.1, info.2, info.3));
                      }
                      acc
                  }
           ) >>
        (ips)));

impl Transport for UDP {
    fn run(&mut self, handle: &mut Handle, i: ChainedIter) {
        let mut buffer = [0; 512];

        let events = self.inotify
            .read_events(&mut buffer)
            .expect("Error while reading inotify events");

        for e in events {
            if e.name.to_str().unwrap().contains("udhcpd.leases") {
                if let Err(e) = self.update_clients_list(handle.get_utc(), handle.get_clock()) {
                    debug!("unable to update client list: {}", e);
                }

                break;
            }
        }

        let mut buffer = Vec::with_capacity(UDP_MAX_SIZE);

        for p in i {
            if p.queueable {
                self.queue.push_back(p.clone());
                self.inactive_buffer.push_front(p.clone());
                continue;
            }

            if buffer.len() + p.payload.len() > UDP_MAX_SIZE {
                self.send_to_all_clients(handle.get_clock(), &buffer);
                buffer.clear();
            }

            buffer.extend(p.payload.iter());
        }

        self.inactive_buffer.truncate(INACTIVE_BUFFER_SIZE);

        trace!("queue size: {}", self.queue.len());
        // drain queue size * 1/freq of all queued items
        let to_drain = ((1_f32 / handle.get_frequency() as f32) * self.queue.len() as f32)
            .ceil() as usize;

        for _ in 0..to_drain {
            let p = self.queue.pop_front().unwrap();

            if buffer.len() + p.payload.len() > UDP_MAX_SIZE {
                self.send_to_all_clients(handle.get_clock(), &buffer);
                buffer.clear();
            }

            buffer.extend(p.payload.iter());
        }

        // if buffer is not empty, and we still have space to squeeze, don't waste
        // it as long as we do not introduce new packets
        // otherwise, send the remaining packet
        if !buffer.is_empty() {
            while !self.queue.is_empty() &&
                  buffer.len() + self.queue.front().unwrap().payload.len() <= UDP_MAX_SIZE {
                let item = self.queue.pop_front().unwrap(); // this can not fail

                buffer.extend(item.payload.iter());
            }

            self.send_to_all_clients(handle.get_clock(), &buffer);
            buffer.clear();
        }

        run_every!(PING_FREQ, self.ping_counter, handle, {
            debug!("sending ping to all clients");

            self.send_icmp_echo_request_to_all_clients();
        });

        self.read_icmp_responses(handle.get_clock());

        let inactive_buffer_len = self.inactive_buffer.len();

        let mut need_replay = HashSet::new();

        for (ip, c) in self.clients.iter_mut() {
            if (handle.get_clock() - c.last_reply).as_secs() > DEAD_THRESHOLD {
                c.active = false;
            } else if !c.active {
                c.active = true;
                c.last_refused = handle.get_clock();
            }

            if c.active {
                if (handle.get_clock() - c.last_refused).as_secs() < IN_APP_THRESHOLD {
                    c.in_app = false;
                } else if !c.in_app {
                    // when iPad is unreachable (sleeping), in_app will appears to be active
                    c.in_app = true;
                    need_replay.insert(ip.clone());
                }
            }
        }

        for ip in need_replay {
            let c = self.clients.get_mut(&ip).unwrap();

            if (handle.get_clock() - c.last_replay).as_secs() < REPLAY_INTERVAL {
                continue;
            }

            c.last_replay = handle.get_clock();

            debug!("client {} came back online, replaying {} queued messages", ip, inactive_buffer_len);

            for p in self.inactive_buffer.iter().rev() {
                if buffer.len() + p.payload.len() > UDP_MAX_SIZE {
                    c.send_payload(&buffer);
                    buffer.clear();
                }

                buffer.extend(p.payload.iter());
            }

            if !buffer.is_empty() {
                c.send_payload(&buffer);
            }
        }
    }
}

impl UDP {
    pub fn new() -> Box<Transport> {
        let mut inotify = Inotify::init().unwrap();
        inotify
            .add_watch(WATCH_PATH, watch_mask::MODIFY | watch_mask::CREATE)
            .unwrap();

        let mut me = Box::new(UDP {
                                  clients: HashMap::new(),
                                  inotify,
                                  queue: VecDeque::new(),
                                  inactive_buffer: VecDeque::with_capacity(INACTIVE_BUFFER_SIZE),
                                  ping_counter: 0,
                              });

        if let Err(e) = me.update_clients_list(now_utc(), Instant::now()) {
            debug!("unable to update client list: {}", e);
        }

        me
    }

    fn read_icmp_responses(&mut self, clock: Instant) {
        let mut buf = [0_u8; 22];

        for (ip, c) in self.clients.iter_mut() {
            if let Ok((n, IpAddr::V4(recv_ip))) = c.icmp_sock.recv_from(&mut buf) {
                if n != buf.len() || &recv_ip != ip {
                    continue;
                }

                if buf[20] == 0 && buf[21] == 0 {
                    trace!("got ICMP echo reply from {}", ip);
                    c.last_reply = clock;
                }
            }
        }
    }

    fn send_icmp_echo_request_to_all_clients(&mut self) {
        for (ip, c) in self.clients.iter_mut() {
            if let Err(e) = c.icmp_sock.send(&PING_PACKET) {
                if e.kind() != ErrorKind::WouldBlock {
                    error!("unable to send ping to {}", ip)
                }
            }
        }
    }

    fn send_to_all_clients(&mut self, clock: Instant, buffer: &[u8]) {
        for (_, c) in self.clients.iter_mut() {
            if let Err(e) = c.udp_sock.send(buffer) {
                if e.kind() == ErrorKind::WouldBlock {
                    warn!("UDP send overwhelming buffers");
                }

                match e.kind() {
                    ErrorKind::WouldBlock => warn!("UDP send overwhelming buffers"),
                    ErrorKind::ConnectionRefused => c.last_refused = clock,
                    _ => error!("UDP send failed: {}", e),
                }
            }
        }
    }

    fn update_clients_list(&mut self, utc: Tm, clock: Instant) -> io::Result<()> {
        let mut buf = Vec::new();
        let mut file = try!(File::open(LEASE_FILE_PATH));

        try!(file.read_to_end(&mut buf));

        if buf.len() > 0 {
            if let IResult::Done(_, mut alive) =
                parse_ip_from_lease_file(&buf[..], utc.to_timespec(), (buf.len() - 8) / 36) {
                debug!("found client IP(s) {:?} from lease file", alive);

                self.clients
                    .retain(|k, _| if alive.contains(k) {
                                // keep sending
                                alive.remove(k);
                                true
                            } else {
                        info!("removing client: {}", k);
                        false
                    });

                // here, we are left with IPs that are not in self.clients yet
                for ip in alive {
                    let udp_sock = UdpSocket::bind("0.0.0.0:0").expect("can not bind UDP socket");
                    udp_sock
                        .set_nonblocking(true)
                        .expect("could not set socket to non blocking mode");
                    if let Err(e) = udp_sock.connect((ip, GDL90_PORT)) {
                        error!("could not connect to client IP: {} (UDP)", e);
                        continue;
                    }

                    let icmp_sock = IcmpSocket::connect(ip.into())
                        .expect("could not connect to ICMP socket");

                    icmp_sock
                        .set_write_timeout(Some(Duration::new(0, 1))) // TODO, fix this once we have real nonblocking mode
                        .unwrap();
                    icmp_sock
                        .set_read_timeout(Some(Duration::new(0, 1))) // TODO, fix this once we have real nonblocking mode
                        .unwrap();

                    self.clients
                        .insert(ip,
                                Client {
                                    udp_sock,
                                    icmp_sock,
                                    active: true,
                                    last_reply: clock,
                                    in_app: false,
                                    last_refused: clock,
                                    last_replay: clock,
                                });

                    info!("new client: {}", ip);
                }
            }
        }

        Ok(())
    }
}

impl Client {
    fn send_payload(&self, buffer: &[u8]) {
        if let Err(e) = self.udp_sock.send(buffer) {
            if e.kind() == ErrorKind::WouldBlock {
                warn!("UDP send overwhelming buffers");
            }
        }
    }
}
