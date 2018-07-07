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

use super::super::*;
use std::collections::VecDeque;
use std::os::raw::c_void;
use std::slice::from_raw_parts;

const ADDR_TYPE_ADS_B_ICAO: u8 = 1;
const ADDR_TYPE_ADS_B_OTHER: u8 = 2;
const ADDR_TYPE_ADS_R_ICAO: u8 = 3;
const ADDR_TYPE_ADS_R_OTHER: u8 = 4;
const ADDR_TYPE_TIS_B_ICAO: u8 = 5;
const ADDR_TYPE_TIS_B_OTHER: u8 = 6;
const ADDR_TYPE_UNKNOWN: u8 = 7;

const SPEED_IS_GS: u8 = 1;
const SPEED_IS_IAS: u8 = 2;
const SPEED_IS_TAS: u8 = 3;

#[derive(Debug)]
#[repr(C)]
struct TrafficT {
    addr: u32,
    altitude: i32,
    gnss_delta: i32,
    heading: u32,
    speed: u32,
    vs: i32,
    squawk: u32,
    callsign: *const u8,
    category: u32,
    lat: f64,
    lon: f64,
    nic: u32,
    nacp: u32,
    on_ground: u8,

    addr_type: u8,
    altitude_valid: u8,
    altitude_is_baro: u8,
    gnss_delta_valid: u8,
    heading_valid: u8,
    heading_is_true: u8,
    speed_valid: u8,
    speed_src: u8,
    vs_valid: u8,
    squawk_valid: u8,
    callsign_valid: u8,
    category_valid: u8,
    pos_valid: u8,
    nacp_valid: u8,
    airground_valid: u8,
}

pub struct Dump1090 {
    parsed: VecDeque<TrafficData>,
}

#[link(name = "dump1090")]
extern "C" {
    fn dump1090_init(
        cb: extern "C" fn(inst: *mut c_void, traffic: *const TrafficT),
        data: *const c_void,
    ) -> i32;
    fn dump1090_process(data: *const u8, len: usize);
}

impl Dump1090 {
    pub fn new() -> Box<Self> {
        // this has to be boxed to get the address of self for callback
        // now
        let me = Box::new(Self {
            parsed: VecDeque::new(),
        });

        unsafe {
            if dump1090_init(callback, &*me as *const _ as *const c_void) != 0 {
                panic!("unable to init libdump1090");
            }
        }

        me
    }

    pub fn process_data(&mut self, buf: &[u8]) {
        unsafe { dump1090_process(buf.as_ptr(), buf.len()) }
    }

    pub fn parsed_as_mut_ref(&mut self) -> &mut VecDeque<TrafficData> {
        &mut self.parsed
    }

    fn push_message(&mut self, msg: TrafficData) {
        trace!("got a Mode S message: {:?}", msg);
        self.parsed.push_back(msg);
    }
}

unsafe impl Send for Dump1090 {}

extern "C" fn callback(inst: *mut c_void, traffic: *const TrafficT) {
    let inst = inst as *mut Dump1090;

    unsafe {
        let traffic = &*traffic;
        if traffic.addr == 0 {
            // this happens sometimes, just ignore
            return;
        }

        let msg = TrafficData {
            addr: (
                traffic.addr,
                match traffic.addr_type {
                    ADDR_TYPE_ADS_B_ICAO => AddressType::ADSBICAO,
                    ADDR_TYPE_ADS_B_OTHER => AddressType::ADSBOther,
                    ADDR_TYPE_ADS_R_ICAO => AddressType::ADSRICAO,
                    ADDR_TYPE_ADS_R_OTHER => AddressType::ADSROther,
                    ADDR_TYPE_TIS_B_ICAO => AddressType::TISBICAO,
                    ADDR_TYPE_TIS_B_OTHER => AddressType::TISBOther,
                    ADDR_TYPE_UNKNOWN => AddressType::Unknown,
                    _ => unreachable!(),
                },
            ),
            altitude: match traffic.altitude_valid {
                1 => Some((
                    traffic.altitude,
                    if traffic.altitude_is_baro == 1 {
                        AltitudeType::Baro
                    } else {
                        AltitudeType::GNSS
                    },
                )),
                _ => None,
            },
            gnss_delta: match traffic.gnss_delta_valid {
                1 => Some(traffic.gnss_delta),
                _ => None,
            },
            heading: match traffic.heading_valid {
                1 => Some((
                    traffic.heading as u16,
                    if traffic.heading_is_true == 1 {
                        HeadingType::True
                    } else {
                        HeadingType::Mag
                    },
                )),
                _ => None,
            },
            speed: match traffic.speed_valid {
                1 => Some((
                    traffic.speed as u16,
                    match traffic.speed_src {
                        SPEED_IS_GS => SpeedType::GS,
                        SPEED_IS_IAS => SpeedType::IAS,
                        SPEED_IS_TAS => SpeedType::TAS,
                        _ => unreachable!(),
                    },
                )),
                _ => None,
            },
            vs: match traffic.vs_valid {
                1 => Some(traffic.vs as i16),
                _ => None,
            },
            squawk: match traffic.squawk_valid {
                1 => {
                    let mut sq = 0_u16;

                    sq += (traffic.squawk as u16 >> 12) * 1000;
                    sq += ((traffic.squawk as u16 & 0x0F00) >> 8) * 100;
                    sq += ((traffic.squawk as u16 & 0x00F0) >> 4) * 10;
                    sq += traffic.squawk as u16 & 0x000F;

                    Some(sq)
                }
                _ => None,
            },
            callsign: match traffic.callsign_valid {
                1 => {
                    let s = from_raw_parts(traffic.callsign, 8);
                    let mut v = Vec::with_capacity(s.len());
                    v.extend_from_slice(s);
                    String::from_utf8(v).ok().and_then(|s| {
                        let trimmed = s.trim();

                        if trimmed.len() > 0 {
                            Some(String::from(trimmed))
                        } else {
                            None
                        }
                    })
                }
                _ => None,
            },
            category: match traffic.category_valid {
                1 => {
                    let mut ct = 0_u8;

                    ct += (((traffic.category as u8 & 0xF0) >> 4) - 0x0A) * 8;
                    ct += traffic.category as u8 & 0x0F;

                    Some(ct)
                }
                _ => None,
            },
            lat_lon: match traffic.pos_valid {
                1 => Some((traffic.lat as f32, traffic.lon as f32)),
                _ => None,
            },
            nic: match traffic.pos_valid {
                1 => Some(traffic.nic as u8),
                _ => None,
            },
            nacp: match traffic.nacp_valid {
                1 => Some(traffic.nacp as u8),
                _ => None,
            },
            on_ground: match traffic.airground_valid {
                1 => Some(traffic.on_ground == 1),
                _ => None,
            },
            source: TrafficSource::ES,
        };

        (*inst).push_message(msg);
    }
}
