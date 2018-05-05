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

pub struct FISB {
    count: usize,
}

impl FISB {
    pub fn new() -> Self {
        Self { count: 0 }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FISBData {
    pub payload: Vec<u8>,
}

impl Processor for FISB {
    fn run(&mut self, handle: &mut Pushable<Report>, i: ChainedIter) {
        for e in i {
            match *e {
                SensorData::FISB(ref p) => {
                    handle.push_data(Report::FISB(p.clone()));
                    self.count += 1;
                }
                _ => {} // do nothing
            }
        }
    }
}
