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
use pitot::handle::Pushable;
use sensor::{Sensor, SensorData};

struct FakeGNSSProvider {}

impl Sensor for FakeGNSSProvider {
    fn run(&mut self, h: &mut Pushable<SensorData>) {
        let fix = SensorData::GNSS(GNSSData::TimeFix {
            time: Some(UTC.ymd(2014, 7, 8).and_hms(9, 10, 11)),
            fix: Some(Fix {
                lat_lon: ((12345_f32, 12345_f32), Some(1000)),
                height_msl: (1000, Some(500)),
                height_ellipsoid: Some((900, Some(500))),
                gs: (10000, Some(100)),
                true_course: (123_f32, Some(2_f32)),
                quality: FixQuality::ThreeDim,
                num_sv: 4,
                mag_dec: Some((10_f32, Some(4_f32))),
            }),
        });

        h.push_data(fix);
    }
}

impl FakeGNSSProvider {
    fn new() -> Option<Box<Self>> {
        Some(Box::new(FakeGNSSProvider {}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pitot::handle::{BasicHandle, PushableHandle};
    use std::collections::VecDeque;

    #[test]
    fn test_fake_gnss_provider() {
        let mut p = FakeGNSSProvider::new().unwrap();
        let mut q = VecDeque::<SensorData>::new();

        for i in 0..2 {
            {
                let mut b = BasicHandle::new(10);
                let mut h = PushableHandle::new(&mut b, &mut q);
                p.run(&mut h);
            }

            assert_eq!(
                q[i],
                SensorData::GNSS(GNSSData::TimeFix {
                    time: Some(UTC.ymd(2014, 7, 8).and_hms(9, 10, 11)),
                    fix: Some(Fix {
                        lat_lon: ((12345_f32, 12345_f32), Some(1000)),
                        height_msl: (1000, Some(500)),
                        height_ellipsoid: Some((900, Some(500))),
                        gs: (10000, Some(100)),
                        true_course: (123_f32, Some(2_f32)),
                        quality: FixQuality::ThreeDim,
                        num_sv: 4,
                        mag_dec: Some((10_f32, Some(4_f32))),
                    }),
                })
            );
        }
    }
}
