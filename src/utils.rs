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

#[macro_export]
macro_rules! mm_to_ft {
    ($x:expr) => (($x as f32) * 0.00328084_f32);
}

#[macro_export]
macro_rules! mmps_to_kts {
    ($x:expr) => (($x as f32) * 0.00194384_f32);
}

#[macro_export]
macro_rules! run_every {
    ($hz:expr, $counter:expr, $handle:expr, $action:block) => {
        $counter += 1;
        if $counter >= ($handle.get_frequency() as f32 / $hz as f32) as u32 {
            $counter = 0;
            $action;
        }
    }
}
