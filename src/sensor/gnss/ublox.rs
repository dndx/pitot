// Pitot - a customizable aviation information receiver
// Copyright (C) 2017-2018  Datong Sun (dndx@idndx.com)
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

use super::*;
use chrono::prelude::*;
use nom::{le_i16, le_i32, le_i8, le_u16, le_u32, le_u8, shift, ErrorKind, IResult};
use pitot::handle::Pushable;
use sensor::{Sensor, SensorData};
use serial::{self, BaudRate, SerialPort, SystemPort};
use std::io::{self, Read, Write};
use std::num::Wrapping;
use std::time::Duration;
use std::{str, thread, time};

const SERIAL_PATH: [&str; 1] = ["/dev/ttyAMA0"];
const BAUD_RATE: BaudRate = BaudRate::Baud38400;

pub struct UbloxGNSSProvider {
    comm: UBXCommunicator,
}

#[derive(Debug, PartialEq)]
struct UBXPacket<'a> {
    class: u8,
    id: u8,
    payload: &'a [u8],
}

#[derive(Debug)]
enum ProtocolError {
    Parse(ErrorKind),
    Checksum,
}

#[derive(Debug)]
enum Error {
    NAK,
    Protocol(ProtocolError),
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<ProtocolError> for Error {
    fn from(err: ProtocolError) -> Error {
        Error::Protocol(err)
    }
}

struct UBXCommunicator {
    /// internal buffer size
    serial: SystemPort,
    v: Vec<u8>,
    /// start of next position
    start: usize,
    /// start of free position
    end: usize,
}

impl UBXCommunicator {
    fn new(serial: SystemPort, buffer_size: usize) -> UBXCommunicator {
        let mut v = Vec::with_capacity(buffer_size);
        v.extend(::std::iter::repeat(0).take(buffer_size));

        UBXCommunicator {
            serial,
            v,
            start: 0,
            end: 0,
        }
    }

    fn refill(&mut self) -> io::Result<usize> {
        shift(&mut self.v, self.start, self.end);
        self.end = self.end - self.start;
        self.start = 0;

        let remaining = &mut self.v[self.end..];
        let end = &mut self.end;

        if remaining.is_empty() {
            return Ok(0);
        }

        self.serial.read(remaining).map(|c| {
            *end += c;
            c
        })
    }

    fn next(&mut self) -> Result<UBXPacket, Error> {
        loop {
            try!(self.refill());

            let s = &self.v[self.start..self.end];
            unsafe {
                // unfortunately this is required because Rust wouldn't be able to infer
                // the lifetime of self correctly otherwise.
                let res = parse_ubx_message(::std::slice::from_raw_parts(s.as_ptr(), s.len()));

                match res {
                    IResult::Done(rem, pkt) => {
                        debug_assert!(rem.len() <= self.end - self.start);
                        self.start = self.end - rem.len();

                        return Ok(pkt);
                    }
                    IResult::Error(e) => {
                        // invalidate buffer
                        self.start = self.end;
                        debug!("ubx parse error, invalidating buffer");
                        return Err(Error::Protocol(ProtocolError::Parse(e)));
                    }
                    IResult::Incomplete(_) => {
                        if self.end - self.start == self.v.len() {
                            warn!("buffer is full but still incomplete");
                            // invalidate buffer
                            self.start = self.end;
                        }

                        continue;
                    }
                }
            }
        }
    }

    /// Write `packet` to wire, wait for ACK/NAK responses if class id is CFG
    fn write(&mut self, packet: &UBXPacket) -> Result<(), Error> {
        try!(
            self.serial
                .write_all(&packet.to_wire())
                .and_then(|_| self.serial.flush())
        );

        let mut n = 0;

        while packet.class == 0x06 && packet.id != 0x00 && packet.payload.len() > 0 {
            // only wait for response if class is CFG and not reconfiguring ports
            // observation is that when CFG-PRT is sent, sometimes we do not even get
            // an ACK/NAK back, thus waiting on it is not really safe to do
            if match self.next() {
                Ok(UBXPacket {
                    class: 0x05,
                    id: 0x01,
                    payload,
                }) if payload == &[packet.class, packet.id] =>
                {
                    return Ok(())
                }
                Ok(UBXPacket {
                    class: 0x05,
                    id: 0x00,
                    payload,
                }) if payload == &[packet.class, packet.id] =>
                {
                    return Err(Error::NAK)
                }
                Ok(_) => {
                    // likely a bad send when switching baudrate
                    n += 1;
                    n % 5 == 0
                }
                Err(Error::Io(ref e)) if e.kind() == io::ErrorKind::TimedOut => {
                    // likely a bad send when switching baudrate
                    n += 1;
                    n % 5 == 0
                }
                Err(Error::Protocol(ProtocolError::Parse(_))) => true,
                Err(e) => return Err(e),
            } {
                // try send again
                debug!("parse error, resending");
                // wait for port to stabilize
                thread::sleep(time::Duration::from_millis(100));
                try!(
                    self.serial
                        .write_all(&packet.to_wire())
                        .and_then(|_| self.serial.flush())
                );
            }
        }

        Ok(())
    }
}

named!(
    parse_ubx_message<UBXPacket>,
    map_res!(
        do_parse!(
            take_until_and_consume!(&[0xB5_u8, 0x62][..])
                >> class: le_u8
                >> id: le_u8
                >> len: le_u16
                >> payload: take!(len)
                >> ck_a: le_u8
                >> ck_b: le_u8
                >> (class, id, payload, ck_a, ck_b)
        ),
        UBXPacket::new_from_parser
    )
);

named!(
    parse_ubx_nav_pvt<GNSSData>, // see p. 291
    map!(
        do_parse!(
            take!(4) >> // skip iTOW
            year: le_u16 >>
            month: le_u8 >>
            day: le_u8 >>
            hour: le_u8 >>
            min: le_u8 >>
            sec: le_u8 >>
            time_valid: le_u8 >>
            take!(4) >> // skip time accuracy
            take!(4) >> // nano sec
            fix_type: le_u8 >>
            fix_status: le_u8 >>
            take!(1) >> // skip flags2 since nothing interesting is in there
            num_sv: le_u8 >>
            lon: le_i32 >>
            lat: le_i32 >>
            height_ellipsoid: le_i32 >>
            height_msl: le_i32 >>
            horizontal_accuracy: le_u32 >>
            vertical_accuracy: le_u32 >>
            take!(4) >> // skip NED north velocity
            take!(4) >> // skip NED east velocity
            take!(4) >> // skip NED down velocity
            gs: le_i32 >>
            hdg: le_i32 >>
            gs_accuracy: le_u32 >>
            hdg_accuracy: le_u32 >>
            take!(2) >> // skip pDOP
            take!(6) >> // skip reserved
            take!(4) >> // skip headVeh
            mag_dec: le_i16 >> mag_dec_accuracy: le_u16
                >> (
                    year,
                    month,
                    day,
                    hour,
                    min,
                    sec,
                    time_valid,
                    fix_type,
                    fix_status,
                    num_sv,
                    lon,
                    lat,
                    height_ellipsoid,
                    height_msl,
                    horizontal_accuracy,
                    vertical_accuracy,
                    gs,
                    hdg,
                    gs_accuracy,
                    hdg_accuracy,
                    mag_dec,
                    mag_dec_accuracy
                )
        ),
        fix_from_pvt
    )
);

named!(
    parse_ubx_nav_sat<GNSSData>, // see p. 296
    map!(
        do_parse!(
            take!(4) >> // skip iTOW
            tag!([0x01]) >> // version = 1
            num_svs: le_u8 >>
            take!(2) >> // skip reserved
            svinfo: count!(
                        map!(do_parse!(
                            gnss_id: le_u8 >>
                            sv_id: le_u8 >>
                            signal: le_u8 >>
                            elev: le_i8 >>
                            azim: le_i16 >>
                            take!(2) >>
                            flags: le_u32 >>
                            (gnss_id, sv_id, signal, elev, azim, flags)
                        ), svinfo_from_protocol)
            , num_svs as usize) >> (svinfo)
        ),
        sat_report_from_svinfo
    )
);

fn svinfo_from_protocol(data: (u8, u8, u8, i8, i16, u32)) -> SVStatus {
    let (gnss_id, sv_id, signal, elev, azim, flags) = data;

    SVStatus {
        system: match gnss_id {
            0 => Constellation::GPS,
            1 => Constellation::SBAS,
            2 => Constellation::Galileo,
            6 => Constellation::GLONASS,
            _ => Constellation::Unknown,
        },
        sv_id,
        signal: Some(signal),
        elevation: Some(elev),
        azimuth: Some(azim as u16),
        healthy: Some(flags & 0x30 != 2),
        acquired: flags & 0x07 >= 2,
        in_solution: flags & 0x08 != 0,
        sbas_in_use: Some(flags & 0x10000 != 0),
    }
}

fn sat_report_from_svinfo(data: Vec<SVStatus>) -> GNSSData {
    GNSSData::SatelliteInfo(data)
}

fn fix_from_pvt(
    data: (
        u16,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        i32,
        i32,
        i32,
        i32,
        u32,
        u32,
        i32,
        i32,
        u32,
        u32,
        i16,
        u16,
    ),
) -> GNSSData {
    let (
        year,
        month,
        day,
        hour,
        min,
        sec,
        time_valid,
        fix_type,
        fix_status,
        num_sv,
        lon,
        lat,
        height_ellipsoid,
        height_msl,
        horizontal_accuracy,
        vertical_accuracy,
        gs,
        hdg,
        gs_accuracy,
        hdg_accuracy,
        mag_dec,
        mag_dec_accuracy,
    ) = data;

    GNSSData::TimeFix {
        time: if time_valid & 0x07 != 0 {
            // validDate || validTime || fullyResolved
            Some(UTC.ymd(year as i32, month as u32, day as u32).and_hms(
                hour as u32,
                min as u32,
                sec as u32,
            ))
        } else {
            // time is unreliable
            None
        },
        fix: if fix_type != 0 && fix_type != 5 {
            Some(super::Fix {
                lat_lon: (
                    (lat as f32 * 1.0e-7, lon as f32 * 1.0e-7),
                    Some(horizontal_accuracy),
                ),
                height_msl: (height_msl, Some(vertical_accuracy)),
                height_ellipsoid: Some((height_ellipsoid, Some(vertical_accuracy))),
                gs: (gs as u32, Some(gs_accuracy)),
                true_course: (hdg as f32 * 1.0e-5, Some(hdg_accuracy as f32 * 1.0e-5)),
                quality: if fix_status & 0x02 != 0 {
                    FixQuality::SBAS
                } else {
                    match fix_type {
                        0 | 5 => unreachable!(),
                        2 => FixQuality::TwoDim,
                        3 => FixQuality::ThreeDim,
                        _ => FixQuality::Unknown,
                    }
                },
                num_sv,
                mag_dec: if time_valid & 0x08 == 0 {
                    None
                } else {
                    Some((
                        mag_dec as f32 * 1.0e-2,
                        Some(mag_dec_accuracy as f32 * 1.0e-2),
                    ))
                },
            })
        } else {
            None
        },
    }
}

impl<'a> UBXPacket<'a> {
    fn new(class: u8, id: u8, payload: &'a [u8]) -> UBXPacket {
        UBXPacket { class, id, payload }
    }

    /// Helper function to map a parser result to UBXPacket struct
    fn new_from_parser(data: (u8, u8, &'a [u8], u8, u8)) -> Result<UBXPacket, Error> {
        let (class, id, payload, ck_a, ck_b) = data;

        let mut to_calc = Vec::with_capacity(payload.len() + 6);
        to_calc.push(class);
        to_calc.push(id);
        // length in LE

        let len_le = payload.len() as u16;
        to_calc.push((len_le & 0xFF) as u8);
        to_calc.push(((len_le >> 8) & 0xFF) as u8);
        to_calc.extend_from_slice(payload);
        let (cck_a, cck_b) = make_ubx_checksum(&to_calc);

        if cck_a != ck_a || cck_b != ck_b {
            debug!("incorrect checksum");
            Err(Error::Protocol(ProtocolError::Checksum))
        } else {
            Ok(UBXPacket::new(class, id, payload))
        }
    }

    /// Returns the representation of `self` to protocol format
    /// suitable for transmitting on a serial connection
    fn to_wire(&self) -> Vec<u8> {
        let mut ret = Vec::with_capacity(self.payload.len() + 8);

        ret.push(0xB5);
        ret.push(0x62);
        ret.push(self.class);
        ret.push(self.id);
        // length in LE

        let len_le = self.payload.len() as u16;
        ret.push((len_le & 0xFF) as u8);
        ret.push(((len_le >> 8) & 0xFF) as u8);
        ret.extend_from_slice(&self.payload);

        let (ck_a, ck_b) = make_ubx_checksum(&ret[2..]);

        ret.push(ck_a);
        ret.push(ck_b);

        ret
    }
}

impl Sensor for UbloxGNSSProvider {
    fn run(&mut self, h: &mut Pushable<SensorData>) {
        loop {
            match self.comm.next() {
                Ok(UBXPacket {
                    class: 0x01,
                    id: 0x07,
                    payload,
                }) => {
                    // PVT
                    let (rem, pvt) = parse_ubx_nav_pvt(payload).unwrap();
                    debug_assert!(rem.len() == 0);
                    trace!("got PVT");
                    h.push_data(SensorData::GNSS(pvt))
                }
                Ok(UBXPacket {
                    class: 0x01,
                    id: 0x35,
                    payload,
                }) => {
                    // SAT
                    let (rem, sat) = parse_ubx_nav_sat(payload).unwrap();
                    debug_assert!(rem.len() == 0);
                    trace!("got SAT");
                    h.push_data(SensorData::GNSS(sat))
                }
                Err(Error::Io(e)) => {
                    if e.kind() == io::ErrorKind::TimedOut {
                        break;
                    } else {
                        info!("I/O error: {:?}, continuing", e);
                        continue;
                    }
                }
                _ => break,
            }
        }
    }
}

impl UbloxGNSSProvider {
    pub fn new() -> Option<Box<Sensor>> {
        for p in &SERIAL_PATH {
            info!("trying port {}", p);
            if let Ok(mut p) = serial::open(p) {
                p.set_timeout(Duration::from_secs(1)).unwrap();
                let mut p = UBXCommunicator::new(p, 1024);

                p.serial
                    .reconfigure(&|settings| {
                        try!(settings.set_baud_rate(BaudRate::Baud9600));
                        settings.set_char_size(serial::Bits8);
                        settings.set_parity(serial::ParityNone);
                        settings.set_stop_bits(serial::Stop1);
                        settings.set_flow_control(serial::FlowNone);
                        Ok(())
                    })
                    .expect("could not configure baud rate");

                // configure port
                // first, set port baud rate

                let payload = &[
                    0x01, // portID
                    0x00, // reserved1
                    0x00,
                    0x00, // txReady
                    0xC0,
                    0x08,
                    0x00,
                    0x00, // mode (UART)
                    0x00,
                    0x96,
                    0x00,
                    0x00, // baudRate (38400)
                    0x01,
                    0x00, // inProtoMask (UBX only)
                    0x01,
                    0x00, // outProtoMask (UBX only)
                    0x00,
                    0x00,
                    0x00,
                    0x00, // flags, padding
                ];
                let packet = UBXPacket::new(0x06, 0x00, payload);
                if let Err(e) = p.write(&packet) {
                    info!(
                        "serial port not responding, Ublox module is disabled: {:?}",
                        e
                    );
                    return None;
                }

                // see https://github.com/dcuddeback/serial-rs/issues/43
                // sleep 50ms to let RPi finishes transmitting
                thread::sleep(time::Duration::from_millis(50));

                p.serial
                    .reconfigure(&|settings| {
                        try!(settings.set_baud_rate(BAUD_RATE));
                        Ok(())
                    })
                    .expect("could not configure baud rate");

                // next, set update rate
                let payload = &[
                    0x64,
                    0x00, // measRate = 100ms
                    0x01,
                    0x00,
                    0x01,
                    0x00, // navRate = 1, timeRef = 1 (GPS)
                ];
                let packet = UBXPacket::new(0x06, 0x08, payload);
                p.write(&packet).expect("could not configure update rate");

                // nav engine settings
                let payload = &mut [0; 36];
                payload[0] = 0x05; // dyn and fixMode
                payload[1] = 0x00;
                payload[2] = 0x07; // dyn = airborne with <2g acceleration
                payload[3] = 0x02; // fixMode = 3D only
                let packet = UBXPacket::new(0x06, 0x24, payload);
                p.write(&packet).expect("could not configure update rate");

                // determine if Galileo is supported
                let galileo_supported;
                let packet = UBXPacket::new(0x0A, 0x04, &[]);
                p.write(&packet).expect("could not pull version");
                loop {
                    match p.next() {
                        Ok(UBXPacket {
                            class: 0x0A,
                            id: 0x04,
                            payload,
                        }) => {
                            info!(
                                "ublox GPS detected, version string: {}",
                                str::from_utf8(payload).unwrap()
                            );
                            // ROM BASE 2.01 (75331)FWVER=SPG 3.01PROTVER=18.00FIS=0xEF4015 (200030)
                            // GPS;GLO;GAL;BDSSBAS;IMES;QZSS
                            galileo_supported =
                                str::from_utf8(&payload[40..]).unwrap().contains(";GAL;");

                            break;
                        }
                        _ => {}
                    }
                }

                let packet = UBXPacket::new(0x06, 0x3E, &[]);
                p.write(&packet).expect("could not pull GNSS configuration");
                loop {
                    match p.next() {
                        Ok(UBXPacket {
                            class: 0x06,
                            id: 0x3E,
                            payload,
                        }) => {
                            info!("hardware tracking channels available: {}", payload[1]);
                            break;
                        }
                        _ => {}
                    }
                }

                let payload = &mut [
                    // see p. 164
                    0x00,
                    0x00,
                    0xFF,
                    0x07, // numTrkChUse = numTrkChHw, numConfigBlocks = 7
                    0x00,
                    0x08,
                    0x10,
                    0x00,
                    0x01,
                    0x00,
                    0x01,
                    0x00, // GPS = 8-16
                    0x01,
                    0x02,
                    0x03,
                    0x00,
                    0x01,
                    0x00,
                    0x01,
                    0x00, // SBAS = 2-3
                    0x02,
                    0x08,
                    0x0E,
                    0x00,
                    0x00,
                    0x00,
                    0x01,
                    0x00, // Galileo = 8-14, disabled
                    0x03,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x01,
                    0x00, // Beidou = disabled
                    0x04,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x01,
                    0x00, // IMES = disabled
                    0x05,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x01,
                    0x00, // QZSS = disabled
                    0x06,
                    0x08,
                    0x0E,
                    0x00,
                    0x01,
                    0x00,
                    0x01,
                    0x00, // Glonass = 8-14
                ];

                if galileo_supported {
                    payload[24] = 0x01;
                    info!("chip supports Galileo");
                }

                let packet = UBXPacket::new(0x06, 0x3E, payload);
                p.write(&packet).expect("could not configure GNSS");

                // SBAS cfg
                // enabled = true, usage = all, maxSBAS = 3, search all PRNs
                let payload = &[0x01, 0x07, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00];
                let packet = UBXPacket::new(0x06, 0x16, payload);
                p.write(&packet).expect("could not configure SBAS");

                // next, enable message (per 1 solution)
                let payload = &[
                    0x01,
                    0x07, // NAV-PVT
                    0x00,
                    0x01,
                    0x00,
                    0x00,
                    0x00,
                    0x00, // DDC, UART1, res, USB, I2C, res
                ];
                let packet = UBXPacket::new(0x06, 0x01, payload);
                p.write(&packet).expect("could not enable PVT message");

                // next, enable SAT (satellite status reporting per 10 solution)
                let payload = &[
                    0x01,
                    0x35, // NAV-SAT
                    0x00,
                    0x0A,
                    0x00,
                    0x00,
                    0x00,
                    0x00, // DDC, UART1, res, USB, I2C, res
                ];
                let packet = UBXPacket::new(0x06, 0x01, payload);
                p.write(&packet).expect("could not enable SAT message");

                // make non-blocking
                p.serial.set_timeout(Duration::from_secs(0)).unwrap();

                return Some(Box::new(UbloxGNSSProvider { comm: p }));
            }
        }

        info!("unable to find any Ublox GPS");

        None
    }
}

/// Given a slice containing the correct range for calculating the checksum,
/// calculate and returns it.
fn make_ubx_checksum(buf: &[u8]) -> (u8, u8) {
    // ublox p. 133
    let mut ck_a = Wrapping(0_u8);
    let mut ck_b = Wrapping(0_u8);

    for b in buf {
        ck_a += Wrapping(*b);
        ck_b += ck_a;
    }

    (ck_a.0, ck_b.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::{ErrorKind, Needed};

    #[test]
    fn test_make_ubx_message() {
        assert_eq!(
            UBXPacket::new(0x0A, 0x04, &[]).to_wire(),
            [0xB5, 0x62, 0x0A, 0x04, 0x00, 0x00, 0x0E, 0x34]
        );
        assert_eq!(
            UBXPacket::new(
                0x06,
                0x24,
                &[
                    0xFF, 0xFF, 0x06, 0x03, 0x00, 0x00, 0x00, 0x00, 0x10, 0x27, 0x00, 0x00, 0x05,
                    0x00, 0xFA, 0x00, 0xFA, 0x00, 0x64, 0x00, 0x2C, 0x01, 0x00, 0x3C, 0x00, 0x00,
                    0x00, 0x00, 0xC8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ]
            ).to_wire(),
            vec![
                0xB5, 0x62, 0x06, 0x24, 0x24, 0x00, 0xFF, 0xFF, 0x06, 0x03, 0x00, 0x00, 0x00, 0x00,
                0x10, 0x27, 0x00, 0x00, 0x05, 0x00, 0xFA, 0x00, 0xFA, 0x00, 0x64, 0x00, 0x2C, 0x01,
                0x00, 0x3C, 0x00, 0x00, 0x00, 0x00, 0xC8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x1A, 0x28,
            ]
        );
    }

    #[test]
    fn test_ubx_parser() {
        let msg = [0xB5, 0x62, 0x0A, 0x04, 0x00, 0x00, 0x0E, 0x34, 0x00];

        assert_eq!(
            parse_ubx_message(&msg),
            IResult::Done(
                &[0x00][..],
                UBXPacket {
                    class: 0x0A,
                    id: 0x04,
                    payload: &[],
                }
            )
        );
        assert_eq!(
            parse_ubx_message(&msg[..7]),
            IResult::Incomplete(Needed::Size(8))
        );

        let msg = [0x00, 0x01, 0xB5, 0x62, 0x0A, 0x04, 0x0F, 0x00, 0x00];
        assert_eq!(
            parse_ubx_message(&msg),
            IResult::Incomplete(Needed::Size(23))
        );

        let msg = [0x00, 0x00];
        assert_eq!(
            parse_ubx_message(&msg),
            IResult::Error(ErrorKind::TakeUntilAndConsume)
        );

        let msg = [
            0xB5, 0x62, 0x0A, 0x04, 0x00, 0x00, 0x0E, 0x34, 0xB5, 0x62, 0x0A, 0x04, 0x00, 0x00,
            0x0E, 0x34,
        ];
        assert_eq!(
            parse_ubx_message(&msg),
            IResult::Done(
                &[0xB5, 0x62, 0x0A, 0x04, 0x00, 0x00, 0x0E, 0x34][..],
                UBXPacket {
                    class: 0x0A,
                    id: 0x04,
                    payload: &[],
                }
            )
        );

        let msg = [
            0x00, 0x01, 0x02, 0xB5, 0x62, 0x0A, 0x04, 0x00, 0x00, 0x0E, 0x34,
        ];

        assert_eq!(
            parse_ubx_message(&msg),
            IResult::Done(
                &[][..],
                UBXPacket {
                    class: 0x0A,
                    id: 0x04,
                    payload: &[],
                }
            )
        );

        let payload = [
            192, 158, 224, 6, 225, 7, 5, 22, 8, 2, 46, 55, 56, 17, 0, 0, 32, 240, 5, 0, 0, 0, 6, 0,
            44, 28, 253, 182, 179, 195, 113, 22, 112, 233, 255, 255, 170, 94, 0, 0, 22, 196, 13, 0,
            232, 187, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 66, 15,
            0, 128, 168, 18, 1, 15, 39, 0, 0, 248, 74, 35, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(
            parse_ubx_nav_pvt(&payload),
            IResult::Done(
                &[][..],
                GNSSData::TimeFix {
                    time: Some(UTC.ymd(2017, 5, 22).and_hms(8, 2, 46)),
                    fix: None,
                }
            )
        );

        let payload = [
            148, 99, 86, 7, 225, 7, 5, 22, 10, 11, 24, 55, 60, 3, 0, 0, 88, 166, 244, 5, 3, 0, 6,
            6, 28, 27, 253, 182, 131, 185, 113, 22, 117, 202, 255, 255, 175, 63, 0, 0, 45, 71, 1,
            0, 91, 36, 7, 0, 150, 253, 255, 255, 47, 1, 0, 0, 117, 0, 0, 0, 176, 2, 0, 0, 0, 0, 0,
            0, 79, 15, 0, 0, 128, 168, 18, 1, 105, 3, 0, 0, 248, 74, 35, 0, 0, 0, 0, 0, 0, 0, 246,
            255,
        ];
        assert_eq!(
            parse_ubx_nav_pvt(&payload),
            IResult::Done(
                &[][..],
                GNSSData::TimeFix {
                    time: Some(UTC.ymd(2017, 5, 22).and_hms(10, 11, 24)),
                    fix: Some(Fix {
                        lat_lon: ((37.65518, -122.492645), Some(83757)),
                        height_msl: (16303, Some(468059)),
                        height_ellipsoid: Some((-13707, Some(468059))),
                        gs: (688, Some(3919)),
                        true_course: (0_f32, Some(180_f32)),
                        quality: FixQuality::ThreeDim,
                        num_sv: 6,
                        mag_dec: None,
                    }),
                }
            )
        );

        // same as above, but SBAS flag is on and mac_dec = 10
        let payload = [
            148, 99, 86, 7, 225, 7, 5, 22, 10, 11, 24, 63, 60, 3, 0, 0, 88, 166, 244, 5, 3, 2, 6,
            6, 28, 27, 253, 182, 131, 185, 113, 22, 117, 202, 255, 255, 175, 63, 0, 0, 45, 71, 1,
            0, 91, 36, 7, 0, 150, 253, 255, 255, 47, 1, 0, 0, 117, 0, 0, 0, 176, 2, 0, 0, 0, 0, 0,
            0, 79, 15, 0, 0, 128, 168, 18, 1, 105, 3, 0, 0, 248, 74, 35, 0, 0, 0, 0, 0, 0, 0, 246,
            255,
        ];
        assert_eq!(
            parse_ubx_nav_pvt(&payload),
            IResult::Done(
                &[][..],
                GNSSData::TimeFix {
                    time: Some(UTC.ymd(2017, 5, 22).and_hms(10, 11, 24)),
                    fix: Some(Fix {
                        lat_lon: ((37.65518, -122.492645), Some(83757)),
                        height_msl: (16303, Some(468059)),
                        height_ellipsoid: Some((-13707, Some(468059))),
                        gs: (688, Some(3919)),
                        true_course: (0_f32, Some(180_f32)),
                        quality: FixQuality::SBAS,
                        num_sv: 6,
                        mag_dec: Some((0_f32, Some(655.26))),
                    }),
                }
            )
        );

        let payload = [
            36, 209, 62, 8, 1, 3, 0, 0, 0, 2, 0, 0, 70, 1, 0, 0, 17, 18, 0, 0, 6, 14, 0, 8, 219, 0,
            0, 0, 17, 18, 0, 0, 6, 88, 12, 33, 20, 0, 0, 0, 44, 0, 1, 0,
        ];
        assert_eq!(
            parse_ubx_nav_sat(&payload),
            IResult::Done(
                &[][..],
                GNSSData::SatelliteInfo(vec![
                    SVStatus {
                        system: Constellation::GPS,
                        sv_id: 2,
                        signal: Some(0),
                        elevation: Some(0),
                        azimuth: Some(326),
                        healthy: Some(true),
                        acquired: false,
                        in_solution: false,
                        sbas_in_use: Some(false),
                    },
                    SVStatus {
                        system: Constellation::GLONASS,
                        sv_id: 14,
                        signal: Some(0),
                        elevation: Some(8),
                        azimuth: Some(219),
                        healthy: Some(true),
                        acquired: false,
                        in_solution: false,
                        sbas_in_use: Some(false),
                    },
                    SVStatus {
                        system: Constellation::GLONASS,
                        sv_id: 88,
                        signal: Some(12),
                        elevation: Some(33),
                        azimuth: Some(20),
                        healthy: Some(true),
                        acquired: true,
                        in_solution: true,
                        sbas_in_use: Some(true),
                    },
                ])
            )
        );
    }
}
