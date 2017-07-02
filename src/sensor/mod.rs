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

pub mod gnss;
pub mod sdr;

use self::sdr::TrafficData;

use pitot::handle::Pushable;
use self::gnss::GNSSData;
use processor::fisb::FISBData;

#[derive(PartialEq, Debug)]
/// Represents data received from the sensor layer
pub enum SensorData {
    GNSS(GNSSData),
    Traffic(TrafficData),
    FISB(FISBData),
}

/// A type for representing a sensor
pub trait Sensor {
    /// Run the provider, may or may not yield any result
    fn run(&mut self, h: &mut Pushable<SensorData>);
}
