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

use time::{now_utc, Tm};
use std::time::Instant;
use std::collections::VecDeque;

pub trait Handle {
    fn get_utc(&self) -> Tm;
    fn get_clock(&self) -> Instant;
    fn get_frequency(&self) -> u16;
}

pub trait Pushable<D>: Handle {
    fn push_data(&mut self, d: D);
}

pub struct BasicHandle {
    utc: Tm,
    clock: Instant,
    freq: u16,
}

impl Handle for BasicHandle {
    fn get_utc(&self) -> Tm {
        self.utc
    }

    fn get_clock(&self) -> Instant {
        self.clock
    }

    fn get_frequency(&self) -> u16 {
        self.freq
    }
}

impl BasicHandle {
    pub fn new(freq: u16) -> Self {
        Self {
            utc: now_utc(),
            clock: Instant::now(),
            freq,
        }
    }
}

pub struct PushableHandle<'a, H, D>
    where D: 'a,
          H: 'a + Handle
{
    handle: &'a mut H,
    queue: &'a mut VecDeque<D>,
}

impl<'a, H, D> Handle for PushableHandle<'a, H, D>
    where H: 'a + Handle
{
    fn get_utc(&self) -> Tm {
        self.handle.get_utc()
    }

    fn get_clock(&self) -> Instant {
        self.handle.get_clock()
    }

    fn get_frequency(&self) -> u16 {
        self.handle.get_frequency()
    }
}

impl<'a, H, D> Pushable<D> for PushableHandle<'a, H, D>
    where H: 'a + Handle
{
    fn push_data(&mut self, d: D) {
        self.queue.push_back(d);
    }
}

impl<'a, H, D> PushableHandle<'a, H, D>
    where H: 'a + Handle
{
    pub fn new(handle: &'a mut H, queue: &'a mut VecDeque<D>) -> Self {
        Self { handle, queue }
    }
}
