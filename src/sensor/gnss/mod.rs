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

use chrono::prelude::*;

mod fake;
pub mod ublox;

#[derive(PartialEq, Debug, Copy, Clone, Serialize)]
pub enum Constellation {
    GPS,
    SBAS,
    Galileo,
    GLONASS,
    Unknown,
}

#[derive(PartialEq, Debug, Copy, Clone, Serialize)]
pub enum FixQuality {
    TwoDim,
    ThreeDim,
    SBAS,
    Unknown,
}

/// Reading represents a mandatory reading, where the first
/// field is the number and second field is the accuracy (optional)
pub type Reading<T, U> = (T, Option<U>);
/// OptionalReading represents an optional reading, where the first
/// field is the number and second field is the accuracy
pub type OptionalReading<T, U> = Option<(T, Option<U>)>;

#[derive(PartialEq, Debug)]
pub struct Fix {
    /// Fix quality
    pub quality: FixQuality,
    /// Number of SVs used in fix
    pub num_sv: u8,
    /// Lat/Lon in degrees and accuracy in millimeters
    pub lat_lon: Reading<(f32, f32), u32>,
    /// Height above MSL and accuracy in millimeters
    pub height_msl: Reading<i32, u32>,
    /// Height above ellipsoid and accuracy in millimeters
    pub height_ellipsoid: Reading<i32, u32>,
    /// Ground speed and accuracy in millimeters per second
    pub gs: Reading<u32, u32>,
    /// True course and accuracy in degrees
    pub true_course: Reading<f32, f32>,
    /// Magnetic declination in degrees, if unknown, use 0
    pub mag_dec: OptionalReading<f32, f32>,
}

#[derive(PartialEq, Debug, Copy, Clone, Serialize)]
pub struct SVStatus {
    /// Constellation this satellite belongs
    system: Constellation,
    /// SVid inside system (not PRN)
    sv_id: u8,
    /// Signal strength in dbHz
    signal: Option<u8>,
    /// Elevation in degrees
    elevation: Option<i8>,
    /// Azimuth in degrees
    azimuth: Option<u16>,
    /// Is this satellite healthy?
    healthy: Option<bool>,
    /// Signal acquired?
    acquired: bool,
    /// In solution?
    in_solution: bool,
    /// SBAS corrections applies to this SV?
    sbas_in_use: Option<bool>,
}

#[derive(PartialEq, Debug)]
pub enum GNSSData {
    /// A position and fix, either time or fix can be None
    /// but not both (as it makes no sense)
    TimeFix {
        /// Time this fix was generated (UTC)
        time: Option<DateTime<UTC>>,
        fix: Option<Fix>,
    },
    /// Satellite status report
    SatelliteInfo(Vec<SVStatus>),
}
