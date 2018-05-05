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

pub mod handle;

use std::collections::VecDeque;
use std::time::{Duration, Instant};
use std::thread::sleep;
use sensor::SensorData;
use sensor::Sensor;
use processor::{Processor, Report};
use protocol::{Payload, Protocol};
use transport::Transport;
use self::handle::{PushableHandle, BasicHandle};

pub struct Pitot {
    sensors: Vec<Box<Sensor>>,
    processors: Vec<Box<Processor>>,
    protocols: Vec<Box<Protocol>>,
    transports: Vec<Box<Transport>>,
    interval: Duration,
    frequency: u16,
    sensor_queue: VecDeque<SensorData>,
    report_queue: VecDeque<Report>,
    payload_queue: VecDeque<Payload>,
    handle: BasicHandle,
}

impl Pitot {
    pub fn new(freq: u16) -> Self {
        Pitot {
            sensors: vec![],
            processors: vec![],
            protocols: vec![],
            transports: vec![],
            sensor_queue: VecDeque::new(),
            report_queue: VecDeque::new(),
            payload_queue: VecDeque::new(),
            frequency: freq,
            interval: Duration::from_millis((1000 / freq) as u64),
            handle: BasicHandle::new(freq),
        }
    }

    pub fn link_sensor(&mut self, s: Box<Sensor>) {
        self.sensors.push(s);
    }

    pub fn link_processor(&mut self, p: Box<Processor>) {
        self.processors.push(p);
    }

    pub fn link_protocol(&mut self, p: Box<Protocol>) {
        self.protocols.push(p);
    }

    pub fn link_transport(&mut self, t: Box<Transport>) {
        self.transports.push(t);
    }

    fn run_sensors(&mut self) {
        let mut handle = PushableHandle::new(&mut self.handle, &mut self.sensor_queue);

        for s in self.sensors.iter_mut() {
            s.run(&mut handle);
        }
    }

    fn run_processors(&mut self) {
        let mut handle = PushableHandle::new(&mut self.handle, &mut self.report_queue);

        {
            let (first, second) = self.sensor_queue.as_slices();
            debug!("total {} sensor message to process",
                   self.sensor_queue.len());
            trace!("{:?}", first);
            trace!("{:?}", second);

            for s in self.processors.iter_mut() {
                s.run(&mut handle, first.iter().chain(second));
            }
        }

        self.sensor_queue.clear();
    }

    fn run_protocols(&mut self) {
        let mut handle = PushableHandle::new(&mut self.handle, &mut self.payload_queue);

        {
            let (first, second) = self.report_queue.as_slices();
            debug!("total {} report message to process",
                   self.report_queue.len());
            trace!("{:?}", first);
            trace!("{:?}", second);

            for s in self.protocols.iter_mut() {
                s.run(&mut handle, first.iter().chain(second));
            }
        }

        self.report_queue.clear();
    }

    fn run_transports(&mut self) {
        {
            let (first, second) = self.payload_queue.as_slices();
            debug!("total {} payload message to process",
                   self.payload_queue.len());
            trace!("{:?}", first);
            trace!("{:?}", second);

            for s in self.transports.iter_mut() {
                s.run(&mut self.handle, first.iter().chain(second));
            }
        }

        self.payload_queue.clear();
    }

    pub fn run(&mut self) {
        loop {
            // main event loop
            let before = Instant::now();

            // update the handle
            self.handle = BasicHandle::new(self.frequency);

            self.run_sensors();
            self.run_processors();
            self.run_protocols();
            self.run_transports();

            let elapsed = before.elapsed();

            if elapsed < self.interval {
                sleep(self.interval - elapsed);
            } else {
                warn!("loop unable to keep up with the set frequency");
            }
        }
    }
}

impl Default for Pitot {
    fn default() -> Pitot {
        Pitot::new(10)
    }
}
