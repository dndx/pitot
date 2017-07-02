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
use std::net::{UdpSocket, Ipv4Addr};
use std::io::{self, ErrorKind, Read};
use std::fs::File;
use std::collections::VecDeque;
use time::{Timespec, Tm, now_utc};
use nom::{IResult, be_u8, be_u32, be_u64};
use inotify::{Inotify, watch_mask};
use super::*;

const LEASE_FILE_PATH: &str = "/tmp/udhcpd.leases";
const WATCH_PATH: &str = "/tmp";
const GDL90_PORT: u16 = 4000;
const UDP_MAX_SIZE: usize = 1472; // maximum UDP payload size without fragmentation in Ethernet environment

pub struct UDP {
    clients: HashMap<Ipv4Addr, UdpSocket>,
    inotify: Inotify,
    queue: VecDeque<Payload>,
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
                if let Err(e) = self.update_clients_list(handle.get_utc()) {
                    debug!("unable to update client list: {}", e);
                }

                break;
            }
        }

        let mut buffer = Vec::with_capacity(UDP_MAX_SIZE);

        for p in i {
            if p.queueable {
                self.queue.push_back(p.clone());
                continue;
            }

            if buffer.len() + p.payload.len() > UDP_MAX_SIZE {
                self.send_to_all_clients(&buffer);
                buffer.clear();
            }

            buffer.extend(p.payload.iter());
        }

        trace!("queue size: {}", self.queue.len());
        // drain queue size * 1/freq of all queued items
        let to_drain = ((1_f32 / handle.get_frequency() as f32) * self.queue.len() as f32)
            .ceil() as usize;

        for _ in 0..to_drain {
            let p = self.queue.pop_front().unwrap();

            if buffer.len() + p.payload.len() > UDP_MAX_SIZE {
                self.send_to_all_clients(&buffer);
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

            self.send_to_all_clients(&buffer);
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
                              });

        if let Err(e) = me.update_clients_list(now_utc()) {
            debug!("unable to update client list: {}", e);
        }

        me
    }

    fn send_to_all_clients(&self, buffer: &[u8]) {
        for (_, c) in self.clients.iter() {
            match c.send(buffer) {
                Err(e) => {
                    if e.kind() == ErrorKind::WouldBlock {
                        warn!("UDP send overwhelming buffers");
                    }
                }
                _ => {}
            }
        }
    }

    fn update_clients_list(&mut self, utc: Tm) -> io::Result<()> {
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
                    let sock = UdpSocket::bind("0.0.0.0:0").expect("can not bind UDP socket");
                    sock.set_nonblocking(true)
                        .expect("could not set socket to non blocking mode");
                    if let Err(e) = sock.connect((ip, GDL90_PORT)) {
                        info!("could not connect to client IP: {}", e);
                        continue;
                    }

                    self.clients.insert(ip, sock);
                    info!("new client: {}", ip);
                }
            }
        }

        Ok(())
    }
}
