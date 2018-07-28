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

use i2cdev::linux::LinuxI2CDevice;
use i2cdev_bmp280::*;
use i2csensors::Barometer;
use pitot::handle::Pushable;
use sensor::{Sensor, SensorData};

const BMP280_I2C_ADDR: u16 = 0x76;
const I2C_DEV: &'static str = "/dev/i2c-1";
const SEA_LEVEL_QNH: f32 = 101.325;

pub struct BMP280BaroProvider {
    bmp280: BMP280<LinuxI2CDevice>,
}

impl BMP280BaroProvider {
    pub fn new() -> Option<Box<Sensor>> {
        let i2c_device = LinuxI2CDevice::new(I2C_DEV, BMP280_I2C_ADDR).unwrap();

        let settings = BMP280Settings {
            compensation: BMP280CompensationAlgorithm::B64,
            t_sb: BMP280Timing::ms0_5,
            iir_filter_coeff: BMP280FilterCoefficient::Medium,
            osrs_t: BMP280TemperatureOversampling::x1,
            osrs_p: BMP280PressureOversampling::StandardResolution,
            power_mode: BMP280PowerMode::NormalMode,
        };

        if let Ok(b) = BMP280::new(i2c_device, settings) {
            Some(Box::new(Self { bmp280: b }))
        } else {
            info!("BMP280 not found!");
            None
        }
    }
}

impl Sensor for BMP280BaroProvider {
    fn run(&mut self, h: &mut Pushable<SensorData>) {
        let pressure = self.bmp280.pressure_kpa().unwrap();

        let altitude = 145366.45 * (1_f32 - (pressure / SEA_LEVEL_QNH).powf(0.190284));

        h.push_data(SensorData::Baro(altitude.round() as i32))
    }
}
