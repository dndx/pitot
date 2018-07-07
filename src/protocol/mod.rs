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

use pitot::handle::Pushable;
use processor::Report;
use std::iter::Chain;
use std::slice::Iter;

type ChainedIter<'a> = Chain<Iter<'a, Report>, Iter<'a, Report>>;

pub mod gdl90;
pub mod websocket;

#[derive(PartialEq, Debug, Clone)]
pub struct Payload {
    pub queueable: bool,
    pub payload: Vec<u8>,
}

pub trait Protocol {
    /// Deliver event `e` to this processor
    fn run(&mut self, handle: &mut Pushable<Payload>, i: ChainedIter);
}
