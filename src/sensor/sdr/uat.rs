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

use std::thread::{spawn, JoinHandle};
use std::io::{self, Read};
use std::sync::mpsc::{channel, Receiver};
use std::f32::consts::PI;
use super::bindings::librtlsdr::{get_device_count, get_device_info, Device, HWInfo};
use super::bindings::libdump978::{Dump978, Frame, Move, FrameType};
use processor::fisb::FISBData;
use sensor::{Sensor, SensorData};
use pitot::handle::Pushable;
use nom::shift;
use super::*;

const TUNER_GAIN: i32 = 480;
const SAMPLE_RATE: i32 = 2083334;
const RTL_FREQ: u32 = 28800000;
const TUNER_FREQ: u32 = 28800000;
const CENTER_FREQ: u32 = 978000000;
const BANDWIDTH: u32 = 1000000;
const RTL_SDR_BUF_SIZE: usize = 16 * 16384;
const CALLSIGN_BASE40: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ  ..";
const LAT_LON_RESOLUTION: f32 = 360.0 / 16777216_f32; // 2^24, see p. 19
const TRACK_RESOLUTION: f32 = 360.0 / 512.0;

pub struct UAT {
    _handle: JoinHandle<()>,
    rx: Receiver<Frame>,
}

impl UAT {
    pub fn new() -> Option<Self> {
        for i in 0..get_device_count() {
            if let Some(HWInfo { serial: ref s, .. }) = get_device_info(i) {
                if !s.contains("978") {
                    continue;
                }

                let mut dev = Device::new(i).unwrap();
                dev.set_tuner_gain_mode(true)
                    .unwrap()
                    .set_tuner_gain(TUNER_GAIN)
                    .unwrap()
                    .set_sample_rate(SAMPLE_RATE)
                    .unwrap()
                    .set_xtal_freq(RTL_FREQ, TUNER_FREQ)
                    .unwrap()
                    .set_center_freq(CENTER_FREQ)
                    .unwrap()
                    .set_tuner_bandwidth(BANDWIDTH)
                    .unwrap()
                    .reset_buffer()
                    .unwrap();

                info!("UAT initialization successful");

                let mut dump978 = Dump978::new();

                let (tx, rx) = channel();

                // this thread is responsible for reading the SDR device and fed
                // dump978
                let handle = spawn(move || {
                    let mut buf = vec![0; RTL_SDR_BUF_SIZE * 2];
                    let mut len = 0_usize;

                    loop {
                        let read_end = len + RTL_SDR_BUF_SIZE;

                        match dev.read(&mut buf[len..read_end]) {
                            Ok(n) => {
                                trace!("UAT read {} bytes", n);
                                len += n;

                                // feed libdump978
                                let Move { start, end } = dump978.process_data(&mut buf[..len]);
                                if start != end {
                                    shift(&mut buf, start, end);
                                    len = end - start;
                                } else {
                                    len = 0;
                                }

                                // process new data
                                let mut acc = 0_usize;
                                while let Some(item) = dump978.parsed_as_mut_ref().pop_front() {
                                    tx.send(item).unwrap();
                                    acc += 1;
                                }

                                debug!("dump978 returned {} frames", acc);
                            }
                            Err(e) => {
                                if e.kind() == io::ErrorKind::TimedOut {
                                    warn!("UAT read timedout");
                                } else {
                                    error!("UAT read error: {}", e.into_inner().unwrap());
                                }
                            }
                        }
                    }
                });

                return Some(UAT {
                                _handle: handle,
                                rx,
                            });
            }
        }

        info!("no UAT device found");

        None
    }
}

fn parse_adsb_downlink(buf: &[u8]) -> TrafficData {
    let mut trfc = TrafficData {
        addr: (((buf[1] as u32) << 16) | ((buf[2] as u32) << 8) | buf[3] as u32,
               match buf[0] & 0x07 {
                   0 => AddressType::ADSBICAO,
                   1 => AddressType::ADSBOther,
                   2 => AddressType::TISBICAO,
                   3 => AddressType::TISBOther,
                   6 => AddressType::ADSRICAO,
                   _ => AddressType::Unknown,
               }),
        altitude: None,
        gnss_delta: None,
        heading: None,
        speed: None,
        vs: None,
        squawk: None,
        callsign: None,
        category: None,
        lat_lon: None,
        nic: None,
        nacp: None,
        on_ground: None,
        source: TrafficSource::UAT,
    };

    let payload_type = (buf[0] & 0xF8) >> 3;

    if payload_type == 1 || payload_type == 3 {
        let b40 = (buf[17] as u16) << 8 | buf[18] as u16;
        trfc.category = Some((b40 / 1600) as u8);

        if (buf[26] >> 1) & 0x01 == 1 {
            // callsign ID = 1
            let mut callsign = String::with_capacity(8);
            let alphabet = CALLSIGN_BASE40.as_bytes();

            callsign.push(alphabet[(b40 % 1600 / 40) as usize] as char);
            callsign.push(alphabet[(b40 % 40) as usize] as char);

            let b40 = (buf[19] as u16) << 8 | buf[20] as u16;
            callsign.push(alphabet[(b40 / 1600) as usize] as char);
            callsign.push(alphabet[(b40 % 1600 / 40) as usize] as char);
            callsign.push(alphabet[(b40 % 40) as usize] as char);

            let b40 = (buf[21] as u16) << 8 | buf[22] as u16;
            callsign.push(alphabet[(b40 / 1600) as usize] as char);
            callsign.push(alphabet[(b40 % 1600 / 40) as usize] as char);
            callsign.push(alphabet[(b40 % 40) as usize] as char);

            let trimmed = callsign.trim();
            if trimmed.len() > 0 {
                trfc.callsign = Some(trimmed.into());
            }
        } else if (buf[23] >> 2) & 0x07 >= 2 {
            // uat_version >= 2
            let mut squawk = 0;

            squawk += b40 % 1600 / 40 * 1000;
            squawk += b40 % 40 * 100;

            let b40 = (buf[19] as u16) << 8 | buf[20] as u16;
            squawk += b40 / 1600 * 10;
            squawk += b40 % 1600 / 40;

            trfc.squawk = Some(squawk);
        }

        trfc.nacp = Some((buf[25] >> 4) & 0x0F);
        // emergency status currently not extracted
    } // type == 1 | 2

    // parse SV (sent in all types of payload)

    trfc.nic = Some(buf[11] & 0x0F);

    if trfc.addr.1 == AddressType::TISBICAO {
        // maybe ADSR?
        if let Some(nic) = trfc.nic {
            if nic >= 7 && trfc.category != None {
                trfc.addr.1 = AddressType::ADSRICAO;
            }
        }
    }

    let raw_lat = (buf[4] as u32) << 15 | (buf[5] as u32) << 7 | buf[6] as u32 >> 1;
    let raw_lon = ((buf[6] & 0x01) as u32) << 23 | (buf[7] as u32) << 15 | (buf[8] as u32) << 7 |
                  buf[9] as u32 >> 1;

    if raw_lat != 0 && raw_lon != 0 {
        let mut lat = raw_lat as f32 * LAT_LON_RESOLUTION;
        if lat > 90_f32 {
            lat -= 180_f32;
        }

        let mut lon = raw_lon as f32 * LAT_LON_RESOLUTION;
        if lon > 180_f32 {
            lon -= 360_f32;
        }
        trfc.lat_lon = Some((lat, lon));
    }

    let raw_alt = (buf[10] as u16) << 4 | (buf[11] as u16 & 0xF0) >> 4;
    if raw_alt != 0 {
        trfc.altitude = Some(((raw_alt as i32 - 1) * 25 - 1000,
                              if buf[9] & 0x01 == 1 {
                                  AltitudeType::GNSS
                              } else {
                                  AltitudeType::Baro
                              }));
    }

    match (buf[12] >> 6) & 0x03 {
        typ @ 0...1 => {
            trfc.on_ground = Some(false);

            let raw_ns = (buf[12] as i16 & 0x1F) << 6 | (buf[13] as i16 & 0xFC) >> 2;
            let raw_ew = (buf[13] as i16 & 0x03) << 9 | (buf[14] as i16) << 1 |
                         (buf[15] as i16 & 0x80) >> 7;

            if raw_ns & 0x3FF != 0 && raw_ew & 0x3FF != 0 {
                let mut ns_vel = (raw_ns & 0x3FF) as i32 - 1;
                let mut ew_vel = (raw_ew & 0x3FF) as i32 - 1;

                if raw_ns & 0x400 != 0 {
                    ns_vel = -ns_vel;
                }

                if raw_ew & 0x400 != 0 {
                    ew_vel = -ew_vel;
                }

                if typ == 1 {
                    // supersonic
                    ns_vel *= 4;
                    ew_vel *= 4;
                }

                trfc.speed = Some((((ns_vel * ns_vel) as f32 + (ew_vel * ew_vel) as f32)
                                       .sqrt()
                                       .round() as u16,
                                   SpeedType::GS));
                if ns_vel != 0 || ew_vel != 0 {
                    let trk = ((360 + 90 -
                                (((ns_vel as f32).atan2(ew_vel as f32) * 180.0 / PI)
                                     .round() as i16)) % 360) as u16;
                    trfc.heading = Some((trk, HeadingType::True));
                }
            }

            let raw_vs = ((buf[15] & 0x7F) as i16) << 4 | (buf[16] & 0xF0) as i16 >> 4;
            if raw_vs & 0x1FF != 0 {
                let mut vs = ((raw_vs & 0x1FF) - 1) * 64;

                if raw_vs & 0x200 != 0 {
                    vs = -vs;
                }

                trfc.vs = Some(vs);
            }
        }
        2 => {
            // on ground
            trfc.on_ground = Some(true);

            let raw_gs = ((buf[12] & 0x1F) as u16) << 6 | (buf[13] & 0xFC) as u16 >> 2;
            if raw_gs != 0 {
                trfc.speed = Some(((raw_gs & 0x3FF) - 1, SpeedType::GS));
            }

            let raw_trk = ((buf[13] & 0x03) as u16) << 9 | (buf[14] as u16) << 1 |
                          (buf[15] & 0x80) as u16 >> 7;
            trfc.heading = Some((((raw_trk & 0x1FF) as f32 * TRACK_RESOLUTION).round() as u16,
                                 match (raw_trk & 0x600) >> 9 {
                                     1 | 3 => HeadingType::True,
                                     2 => HeadingType::Mag,
                                     _ => HeadingType::True, // assume true
                                 }));
        }
        st => warn!("unknown A/C status: {}", st),
    }

    return trfc;
}

impl Sensor for UAT {
    fn run(&mut self, h: &mut Pushable<SensorData>) {
        for u in self.rx.try_iter() {
            trace!("UAT: {:?}", u);

            match u.frame_type {
                FrameType::GroundUplink => {
                    h.push_data(SensorData::FISB(FISBData { payload: u.payload }))
                }
                FrameType::ADSBShort | FrameType::ADSBLong => {
                    h.push_data(SensorData::Traffic(parse_adsb_downlink(&u.payload)))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_adsb_downlink() {
        let payload = [11, 43, 3, 200, 53, 69, 117, 82, 61, 248, 8, 22, 16, 238, 31, 192, 17, 5,
                       196, 230, 196, 230, 196, 10, 218, 130, 3, 0, 0, 0, 0, 0, 0, 0];
        let exp = TrafficData {
            addr: (0x2B03C8, AddressType::TISBOther),
            altitude: Some((2200, AltitudeType::Baro)),
            gnss_delta: None,
            heading: Some((227, HeadingType::True)),
            speed: Some((85, SpeedType::GS)),
            vs: Some(0),
            squawk: None,
            callsign: None,
            category: Some(0),
            lat_lon: Some((37.456383, -122.17355)),
            nic: Some(6),
            nacp: Some(8),
            on_ground: Some(false),
            source: TrafficSource::UAT,
        };
        assert_eq!(parse_adsb_downlink(&payload), exp);

        let payload = [8, 166, 98, 159, 46, 182, 45, 99, 174, 214, 28, 42, 0, 30, 44, 128, 24, 9,
                       229, 187, 168, 230, 196, 6, 120, 160, 130, 0, 0, 28, 96, 0, 0, 0];
        let exp = TrafficData {
            addr: (0xA6629F, AddressType::ADSBICAO),
            altitude: Some((10225, AltitudeType::Baro)),
            gnss_delta: None,
            heading: Some((274, HeadingType::True)),
            speed: Some((88, SpeedType::GS)),
            vs: Some(0),
            squawk: None,
            callsign: Some(String::from("NDU10")),
            category: Some(1),
            lat_lon: Some((32.844100, -109.91043)),
            nic: Some(10),
            nacp: Some(10),
            on_ground: Some(false),
            source: TrafficSource::UAT,
        };
        assert_eq!(parse_adsb_downlink(&payload), exp);

        let payload = [8, 165, 16, 171, 63, 198, 127, 123, 20, 102, 6, 169, 16, 168, 61, 160, 40,
                       6, 229, 19, 93, 237, 45, 11, 230, 164, 192, 160, 0, 6, 224, 0, 0, 0];
        let exp = TrafficData {
            addr: (0xA510AB, AddressType::ADSBICAO),
            altitude: Some((1625, AltitudeType::Baro)),
            gnss_delta: None,
            heading: Some((109, HeadingType::True)),
            speed: Some((129, SpeedType::GS)),
            vs: Some(-64),
            squawk: Some(4533),
            callsign: None,
            category: Some(1),
            lat_lon: Some((44.842050, -93.459595)),
            nic: Some(9),
            nacp: Some(10),
            on_ground: Some(false),
            source: TrafficSource::UAT,
        };
        assert_eq!(parse_adsb_downlink(&payload), exp);

        let payload = [10, 163, 166, 85, 63, 125, 231, 123, 194, 150, 7, 32, 1, 170, 10, 64, 223,
                       9, 219, 19, 125, 68, 68, 8, 200, 145, 194, 160, 0, 7, 144, 0, 0, 0];
        let exp = TrafficData {
            addr: (0xA3A655, AddressType::TISBICAO),
            altitude: Some((1825, AltitudeType::Baro)),
            gnss_delta: None,
            heading: Some((350, HeadingType::True)),
            speed: Some((107, SpeedType::GS)),
            vs: Some(768),
            squawk: None,
            callsign: Some(String::from("N334TA")),
            category: Some(1),
            lat_lon: Some((44.642665, -92.98117)),
            nic: Some(0),
            nacp: Some(9),
            on_ground: Some(false),
            source: TrafficSource::UAT,
        };
        assert_eq!(parse_adsb_downlink(&payload), exp);
    }
}
