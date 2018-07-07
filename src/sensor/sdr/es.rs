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

use super::bindings::libdump1090::Dump1090;
use super::bindings::librtlsdr::{get_device_count, get_device_info, Device, HWInfo};
use super::*;
use pitot::handle::Pushable;
use sensor::{Sensor, SensorData};
use std::io::{self, Read};
use std::sync::mpsc::{channel, Receiver};
use std::thread::{spawn, JoinHandle};

const TUNER_GAIN: i32 = 480;
const SAMPLE_RATE: i32 = 2400000;
const CENTER_FREQ: u32 = 1090000000;
const RTL_SDR_BUF_SIZE: usize = 16 * 16384;

pub struct ES {
    _handle: JoinHandle<()>,
    rx: Receiver<TrafficData>,
}

impl ES {
    pub fn new() -> Option<Self> {
        for i in 0..get_device_count() {
            if let Some(HWInfo { serial: ref s, .. }) = get_device_info(i) {
                if !s.contains("1090") {
                    continue;
                }

                let mut dev = Device::new(i).unwrap();
                dev.set_tuner_gain_mode(true)
                    .unwrap()
                    .set_tuner_gain(TUNER_GAIN)
                    .unwrap()
                    .set_sample_rate(SAMPLE_RATE)
                    .unwrap()
                    .set_center_freq(CENTER_FREQ)
                    .unwrap()
                    .reset_buffer()
                    .unwrap();

                info!("1090ES initialization successful");

                let mut dump1090 = Dump1090::new();

                let (tx, rx) = channel();

                // this thread is responsible for reading the SDR device and fed
                // dump1090
                let handle = spawn(move || {
                    let mut buf = vec![0; RTL_SDR_BUF_SIZE];

                    loop {
                        match dev.read(&mut buf[..]) {
                            Ok(n) => {
                                trace!("ES read {} bytes", n);

                                // feed libdump1090
                                dump1090.process_data(&buf[..]);

                                // process new data
                                let mut acc = 0_usize;
                                while let Some(item) = dump1090.parsed_as_mut_ref().pop_front() {
                                    tx.send(item).unwrap();
                                    acc += 1;
                                }

                                debug!("dump1090 returned {} messages", acc);
                            }
                            Err(e) => {
                                if e.kind() == io::ErrorKind::TimedOut {
                                    warn!("ES read timedout");
                                } else {
                                    error!("ES read error: {}", e.into_inner().unwrap());
                                }
                            }
                        }
                    }
                });

                return Some(ES {
                    _handle: handle,
                    rx,
                });
            }
        }

        info!("no 1090ES device found");

        None
    }
}

impl Sensor for ES {
    fn run(&mut self, h: &mut Pushable<SensorData>) {
        for u in self.rx.try_iter() {
            h.push_data(SensorData::Traffic(u));
        }
    }
}
