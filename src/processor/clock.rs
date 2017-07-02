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

//! Watches GNSS time (if valid) and set system (wall) clock if it gets too far off.

use super::*;
use sensor::gnss::GNSSData;
use libc::{clock_settime, timespec, CLOCK_REALTIME};

// max 3 second tolerance
const MAX_TOLERANCE: i64 = 2;

pub struct Clock;

impl Clock {
    pub fn new() -> Self {
        Self {}
    }
}

impl Processor for Clock {
    #[cfg(target_pointer_width = "64")]
    fn run(&mut self, handle: &mut Pushable<Report>, i: ChainedIter) {
        for e in i {
            match *e {
                SensorData::GNSS(GNSSData::TimeFix { time: Some(ref f), .. }) => {
                    if (handle.get_utc().to_timespec().sec - f.timestamp()).abs() > MAX_TOLERANCE {
                        info!("setting system clock");

                        let ts = timespec {
                            tv_sec: f.timestamp(),
                            tv_nsec: 0,
                        };
                        unsafe {
                            if clock_settime(CLOCK_REALTIME, &ts) != 0 {
                                error!("failed to set system clock");
                            }
                        }

                        break;
                    }
                }
                _ => {} // do nothing
            }
        }
    }

    #[cfg(target_pointer_width = "32")]
    fn run(&mut self, handle: &mut Pushable<Report>, i: ChainedIter) {
        for e in i {
            match *e {
                SensorData::GNSS(GNSSData::TimeFix { time: Some(ref f), .. }) => {
                    if (handle.get_utc().to_timespec().sec - f.timestamp()).abs() > MAX_TOLERANCE {
                        info!("setting system clock");

                        let ts = timespec {
                            tv_sec: f.timestamp() as i32,
                            tv_nsec: 0,
                        };
                        unsafe {
                            if clock_settime(CLOCK_REALTIME, &ts) != 0 {
                                error!("failed to set system clock");
                            }
                        }

                        break;
                    }
                }
                _ => {} // do nothing
            }
        }
    }
}
