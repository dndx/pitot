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

pub mod bindings;
pub mod uat;
pub mod es;

use processor::traffic::{AddressType, SpeedType, AltitudeType, HeadingType, TrafficSource};

#[derive(Debug, PartialEq)]
pub struct TrafficData {
    pub addr: (u32, AddressType),
    pub altitude: Option<(i32, AltitudeType)>,
    pub gnss_delta: Option<i32>,
    pub heading: Option<(u16, HeadingType)>,
    pub speed: Option<(u16, SpeedType)>,
    pub vs: Option<i16>,
    pub squawk: Option<u16>,
    pub callsign: Option<String>,
    pub category: Option<u8>,
    pub lat_lon: Option<(f32, f32)>,
    pub nic: Option<u8>,
    pub nacp: Option<u8>,
    pub on_ground: Option<bool>,
    pub source: TrafficSource,
}
