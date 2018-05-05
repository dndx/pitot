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

pub mod ownship;
pub mod clock;
pub mod traffic;
pub mod fisb;

use sensor::SensorData;
use pitot::handle::Pushable;
use std::iter::Chain;
use std::slice::Iter;

#[derive(Debug)]
pub enum Report {
    Ownship(ownship::Ownship),
    Traffic(traffic::Target),
    FISB(fisb::FISBData),
}

type ChainedIter<'a> = Chain<Iter<'a, SensorData>, Iter<'a, SensorData>>;

/// A `Processor` takes in input from the sensor layer and
/// generates `Report` as necessary for the next layer
pub trait Processor {
    /// Deliver sensor data `e` to this processor
    fn run(&mut self, handle: &mut Pushable<Report>, i: ChainedIter);
}
