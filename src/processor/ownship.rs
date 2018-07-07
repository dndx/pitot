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
use sensor::gnss::GNSSData;
use sensor::SensorData;

#[derive(PartialEq, Debug, Default, Copy, Clone, Serialize)]
pub struct Ownship {
    pub valid: bool,
    /// Latitude in deg
    pub lat: f32,
    /// Longitude in deg
    pub lon: f32,
    /// Height above WGS-84 ellipsoid if available, otherwise MSL in ft
    pub altitude: i32,
    /// NIC
    pub nic: u8,
    /// NACp
    pub nacp: u8,
    /// Ground speed in kts
    pub gs: f32,
    pub track: f32,
}

impl Processor for Ownship {
    fn run(&mut self, handle: &mut Pushable<Report>, i: ChainedIter) {
        for e in i {
            match *e {
                SensorData::GNSS(GNSSData::TimeFix {
                    fix: Some(ref f), ..
                }) => {
                    if let Some(acc) = f.lat_lon.1 {
                        self.nic = 9;
                        self.nacp = match acc as f32 / 1000_f32 {
                            n if n < 3_f32 => 11,
                            n if n < 10_f32 => 10,
                            n if n < 30_f32 => 9,
                            n if n < 92.6 => 8,
                            n if n < 185.2 => 7,
                            n if n < 555.6 => 6,
                            _ => 0,
                        };
                    } else {
                        self.nic = 0;
                        self.nacp = 0;
                    }

                    self.lat = (f.lat_lon.0).0;
                    self.lon = (f.lat_lon.0).1;
                    // GDL90 says we should use height above ellipsoid
                    // here, but most EFB displays as MSL
                    self.altitude = mm_to_ft!(f.height_msl.0).round() as i32;

                    self.gs = mmps_to_kts!(f.gs.0);
                    self.track = f.true_course.0;

                    self.valid = true;

                    handle.push_data(Report::Ownship(*self));
                }
                _ => {} // do nothing
            }
        }
    }
}

impl Ownship {
    pub fn new() -> Box<Processor> {
        Box::new(Ownship::default())
    }
}
