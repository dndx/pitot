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
use sensor::gnss::{FixQuality, GNSSData, SVStatus};

#[derive(Debug, Clone, Serialize)]
pub struct GNSS {
    pub quality: FixQuality,
    pub num_sv: u8,
    pub sv_status: Vec<SVStatus>,
}

impl Processor for GNSS {
    fn run(&mut self, handle: &mut Pushable<Report>, i: ChainedIter) {
        for e in i {
            match *e {
                SensorData::GNSS(GNSSData::TimeFix {
                    fix: Some(ref f), ..
                }) => {
                    self.quality = f.quality;
                    self.num_sv = f.num_sv;
                }
                SensorData::GNSS(GNSSData::SatelliteInfo(ref s)) => {
                    self.sv_status = s.to_vec();
                    handle.push_data(Report::GNSS(self.clone()));
                }
                _ => {} // do nothing
            }
        }
    }
}

impl GNSS {
    pub fn new() -> Self {
        Self {
            quality: FixQuality::Unknown,
            sv_status: Vec::new(),
            num_sv: 0,
        }
    }
}
