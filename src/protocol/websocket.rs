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

use super::*;
use std::thread::{spawn, JoinHandle};
use processor::Report::Ownship;
use ws;
use serde_json;

pub struct WebSocket {
    ws_broadcaster: ws::Sender,
    _handle: JoinHandle<()>,
}

impl WebSocket {
    pub fn new(addr: String) -> Box<Self> {
        // spawn WS thread

        let socket = ws::WebSocket::new(|_| {
            move |_| {
                panic!("This server cannot receive messages, it only sends them.")
            }
        }).expect("Unable to create WebSocket");

        let ws_broadcaster = socket.broadcaster();

        let handle = spawn(move || { socket.listen(addr).expect("Unable to run WebSocket."); });
        debug!("spawned WebSocket thread");

        Box::new(Self {
                     _handle: handle,
                     ws_broadcaster,
                 })
    }
}

impl Protocol for WebSocket {
    fn run(&mut self, _handle: &mut Pushable<Payload>, i: ChainedIter) {
        for r in i {
            match *r {
                Ownship(ref o) => {
                    let mut js = serde_json::to_value(o).unwrap();
                    js["type"] = "Ownship".into();

                    self.ws_broadcaster.send(js.to_string()).unwrap();
                }
                _ => {}
            }
        }
    }
}
