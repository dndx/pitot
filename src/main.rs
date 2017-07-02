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

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate chrono;
extern crate serial;
#[macro_use]
extern crate nom;
extern crate time;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate ws;
extern crate libc;
extern crate inotify;

#[macro_use]
mod utils;
mod sensor;
mod processor;
mod transport;
mod protocol;
mod pitot;

use pitot::Pitot;
use sensor::Sensor;
use processor::Processor;

fn main() {
    env_logger::init().unwrap();

    let mut p = Pitot::new(10); // 10 Hz

    sensor::gnss::ublox::UbloxGNSSProvider::new().and_then(&mut |g| {
                                                                    p.link_sensor(g);
                                                                    Some(())
                                                                });
    sensor::sdr::es::ES::new().and_then(&mut |e| {
                                                 p.link_sensor(Box::new(e) as Box<Sensor>);
                                                 Some(())
                                             });
    sensor::sdr::uat::UAT::new().and_then(&mut |e| {
                                                   p.link_sensor(Box::new(e) as Box<Sensor>);
                                                   Some(())
                                               });

    p.link_processor(processor::ownship::Ownship::new());
    p.link_processor(Box::new(processor::clock::Clock::new()) as Box<Processor>);
    p.link_processor(Box::new(processor::traffic::Traffic::new()) as Box<Processor>);
    p.link_processor(Box::new(processor::fisb::FISB::new()) as Box<Processor>);

    p.link_protocol(protocol::gdl90::GDL90::new());
    p.link_protocol(protocol::websocket::WebSocket::new("0.0.0.0:9001".to_string()));

    p.link_transport(transport::udp::UDP::new());

    p.run();
}
