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

//! Maintains the traffic situation around us.

use super::*;
use std::time::Instant;
use std::collections::HashMap;
use sensor::SensorData;

const CLEANUP_FREQ: f32 = 0.1;
const MAX_STALE_SECS: u64 = 60;
const REPORT_FREQ: u16 = 1;
const LIMITED_ALPHABET: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ";
// lock out TIS-B and ADS-R updates if ADS-B data is less than
// ADS_B_LOCKOUT_INTERVAL seconds old
const ADS_B_LOCKOUT_INTERVAL: u64 = 2;
const FRESHNESS_DELAY: u64 = 6;

pub struct Traffic {
    situation: HashMap<u32, Target>,
    cleanup_counter: u32,
    report_counter: u32,
}

type Address = (u32, AddressType);

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum AddressType {
    ADSBICAO,
    ADSBOther,
    ADSRICAO,
    ADSROther,
    TISBICAO,
    TISBOther,
    Unknown,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum SpeedType {
    GS,
    IAS,
    TAS,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum AltitudeType {
    Baro,
    GNSS,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum HeadingType {
    True,
    Mag,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TrafficSource {
    UAT,
    ES,
}

/// A tracked traffic target
/// Way fields works: `(data, data_type?, last_updated)`
/// Notice that `last_updated` is represented as [`Instant`]
/// which means it is not affected by system clock jumps.
/// Items that does not change often does not have the timestamp stored.
#[derive(Debug, Clone)]
pub struct Target {
    pub addr: Address,
    pub altitude: Option<(i32, AltitudeType, Instant)>,
    pub gnss_delta: Option<i32>,
    pub heading: Option<(u16, HeadingType, Instant)>,
    pub speed: Option<(u16, SpeedType, Instant)>,
    pub vs: Option<(i16, Instant)>,
    pub squawk: Option<u16>,
    pub callsign: Option<String>,
    pub category: Option<u8>,
    pub lat_lon: Option<((f32, f32), Instant)>,
    pub nic: Option<u8>,
    pub nacp: Option<u8>,
    pub on_ground: Option<bool>,
    pub last_seen: Instant,
    pub source: TrafficSource,
}

impl Target {
    pub fn new(addr: Address,
               clock: Instant,
               source: TrafficSource,
               callsign: Option<String>)
               -> Self {
        Target {
            addr,
            altitude: None,
            gnss_delta: None,
            heading: None,
            speed: None,
            vs: None,
            squawk: None,
            callsign,
            category: None,
            lat_lon: None,
            nic: None,
            nacp: None,
            on_ground: None,
            last_seen: clock,
            source: source,
        }
    }

    /// This function literally determines whether the traffic
    /// is worth being passed to upper layer.
    fn is_fresh(&self, now: Instant) -> bool {
        if let Some((_, _, i)) = self.altitude {
            if (now - i).as_secs() <= FRESHNESS_DELAY {
                return true;
            }
        }

        if let Some((_, _, i)) = self.heading {
            if (now - i).as_secs() <= FRESHNESS_DELAY {
                return true;
            }
        }

        if let Some((_, _, i)) = self.speed {
            if (now - i).as_secs() <= FRESHNESS_DELAY {
                return true;
            }
        }

        if let Some((_, i)) = self.lat_lon {
            if (now - i).as_secs() <= FRESHNESS_DELAY {
                return true;
            }
        }

        false
    }
}

impl Traffic {
    pub fn new() -> Self {
        // 100 should be a good start
        Self {
            situation: HashMap::with_capacity(100),
            cleanup_counter: 0,
            report_counter: 0,
        }
    }
}

impl Processor for Traffic {
    fn run(&mut self, handle: &mut Pushable<Report>, i: ChainedIter) {
        let clock = handle.get_clock();

        for e in i {
            // only interested in traffic updates
            match *e {
                SensorData::Traffic(ref t) => {
                    // got a traffic update, first figure out if we have some info
                    // about this guy already

                    let trfc =
                        self.situation
                            .entry(t.addr.0)
                            .or_insert(Target::new(t.addr,
                                                   clock,
                                                   t.source,
                                                   icao_to_tail(t.addr.0)));
                    // here, the callsign will be overwritten by codes below
                    // if it does exist

                    // optimization: if we are also receiving direct ADS-B transmission
                    // from the A/C but this update is ADS-R or TIS-B, ignore it.
                    if (trfc.addr.1 == AddressType::ADSBICAO ||
                        trfc.addr.1 == AddressType::ADSBOther) &&
                       (t.addr.1 != AddressType::ADSBICAO && t.addr.1 != AddressType::ADSBOther) &&
                       (clock - trfc.last_seen).as_secs() < ADS_B_LOCKOUT_INTERVAL {
                        debug!("TIS-B or ADS-R traffic skipped in favor of ADS-B");
                        continue;
                    }

                    // nothing complex here, copy over each changed value and
                    // update the instant

                    trfc.addr = t.addr;
                    trfc.last_seen = clock;
                    trfc.source = t.source;

                    if let Some((alt, typ)) = t.altitude {
                        trfc.altitude = Some((alt, typ, clock));
                    }

                    if let Some(alt) = t.gnss_delta {
                        trfc.gnss_delta = Some(alt);
                    }

                    if let Some((hdg, typ)) = t.heading {
                        trfc.heading = Some((hdg, typ, clock));
                    }

                    if let Some((spd, typ)) = t.speed {
                        trfc.speed = Some((spd, typ, clock));
                    }

                    if let Some(vs) = t.vs {
                        trfc.vs = Some((vs, clock));
                    }

                    if let Some(sq) = t.squawk {
                        trfc.squawk = Some(sq);
                    }

                    if let Some(ref cs) = t.callsign {
                        trfc.callsign = Some(cs.clone());
                    }

                    if let Some(cat) = t.category {
                        trfc.category = Some(cat);
                    }

                    if let Some(ll) = t.lat_lon {
                        trfc.lat_lon = Some((ll, clock));
                    }

                    if let Some(n) = t.nic {
                        trfc.nic = Some(n);
                    }

                    if let Some(na) = t.nacp {
                        trfc.nacp = Some(na);
                    }

                    if let Some(on_gnd) = t.on_ground {
                        trfc.on_ground = Some(on_gnd);
                    }
                }
                _ => {}
            }
        }

        run_every!(CLEANUP_FREQ, self.cleanup_counter, handle, {
            debug!("clean up traffic map");

            self.situation
                .retain(|_, ref v| (clock - v.last_seen).as_secs() < MAX_STALE_SECS);
        });

        run_every!(REPORT_FREQ, self.report_counter, handle, {
            for v in self.situation.values() {
                if v.is_fresh(clock) {
                    handle.push_data(Report::Traffic(v.clone()));
                    trace!("Traffic: {:?}", v);
                }
            }
        });
    }
}

fn n_letters(mut rem: u32, reg: &mut String) {
    if rem == 0 {
        return;
    }

    rem -= 1;
    reg.push(LIMITED_ALPHABET.as_bytes()[(rem / 25) as usize] as char);

    rem %= 25;
    if rem == 0 {
        return;
    }
    rem -= 1;

    reg.push(LIMITED_ALPHABET.as_bytes()[rem as usize] as char);
}

// from https://github.com/cyoung/stratux/blob/master/main/traffic.go#L1177
fn icao_to_tail(icao: u32) -> Option<String> {
    match icao {
        0xA00001...0xAFFFFF => {
            if icao > 0xADF7C7 {
                Some(String::from("US-MIL"))
            } else {
                let mut res = String::with_capacity(6); // N12345
                res.push('N');

                let mut offset = icao - 0xA00001;
                if offset > 915399 {
                    return None;
                }

                res.push((offset / 101711 + 1 + '0' as u32) as u8 as char);
                offset %= 101711;
                if offset <= 600 {
                    n_letters(offset, &mut res);
                    return Some(res);
                }

                offset -= 601;

                res.push((offset / 10111 + '0' as u32) as u8 as char);
                offset %= 10111;
                if offset <= 600 {
                    n_letters(offset, &mut res);
                    return Some(res);
                }

                offset -= 601;

                res.push((offset / 951 + '0' as u32) as u8 as char);
                offset %= 951;
                if offset <= 600 {
                    n_letters(offset, &mut res);
                    return Some(res);
                }

                offset -= 601;

                res.push((offset / 35 + '0' as u32) as u8 as char);
                offset %= 35;
                if offset <= 24 {
                    if offset != 0 {
                        res.push(LIMITED_ALPHABET.as_bytes()[(offset - 1) as usize] as char);
                    }
                    return Some(res);
                }

                offset -= 25;
                res.push((offset + '0' as u32) as u8 as char);

                return Some(res);
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icao_to_tail() {
        assert_eq!(icao_to_tail(0xAA5694), Some(String::from("N76508")));
        assert_eq!(icao_to_tail(0xA29CBF), Some(String::from("N268AK")));
        assert_eq!(icao_to_tail(0xA66A54), Some(String::from("N512R")));
        assert_eq!(icao_to_tail(0xA00001), Some(String::from("N1")));
        assert_eq!(icao_to_tail(0xA029D9), Some(String::from("N11")));
        assert_eq!(icao_to_tail(0xA18FA9), Some(String::from("N20")));
        assert_eq!(icao_to_tail(0x780A2C), None);
    }
}
