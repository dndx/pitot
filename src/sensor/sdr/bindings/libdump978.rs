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

use std::ptr;
use std::slice::from_raw_parts;
use std::collections::VecDeque;
use std::os::raw::c_void;

const ADS_B_SHORT: i32 = 1;
const ADS_B_LONG: i32 = 2;
const GROUND_UPLINK: i32 = 3;

const ADS_B_SHORT_LEN: usize = 18;
const ADS_B_LONG_LEN: usize = 34;
const GROUND_UPLINK_LEN: usize = 432;

enum Dump978T {}

#[repr(C)]
pub struct Move {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, PartialEq)]
pub enum FrameType {
    ADSBShort,
    ADSBLong,
    GroundUplink,
}

#[derive(Debug)]
pub struct Frame {
    pub frame_type: FrameType,
    pub payload: Vec<u8>,
    pub rs_error: u32,
}

pub struct Dump978 {
    ctx: *const Dump978T,
    parsed: VecDeque<Frame>,
}

#[link(name = "dump978")]
extern "C" {
    fn dump978_init(ctx: *mut *const Dump978T,
                    cb: extern "C" fn(inst: *mut c_void,
                                      frame_type: i32,
                                      payload: *const u8,
                                      rs: i32),
                    data: *const c_void)
                    -> i32;
    fn dump978_destroy(ctx: *const Dump978T) -> i32;
    fn dump978_process(ctx: *const Dump978T, data: *mut u8, len: usize) -> Move;
}

impl Dump978 {
    pub fn new() -> Box<Self> {
        // this has to be boxed to get the address of self for callback
        // now
        let mut me = Box::new(Self {
                                  ctx: ptr::null(),
                                  parsed: VecDeque::new(),
                              });

        unsafe {
            if dump978_init(&mut me.ctx, callback, &*me as *const _ as *const c_void) != 0 {
                panic!("unable to init libdump978");
            }
        }

        me
    }

    pub fn destroy(&mut self) {
        if !self.ctx.is_null() {
            unsafe {
                if dump978_destroy(self.ctx) != 0 {
                    panic!("unable to destroy libdump978");
                }

                self.ctx = ptr::null();
            }
        }
    }

    pub fn process_data(&mut self, buf: &mut [u8]) -> Move {
        unsafe { dump978_process(self.ctx, buf.as_mut_ptr(), buf.len()) }
    }

    pub fn parsed_as_mut_ref(&mut self) -> &mut VecDeque<Frame> {
        &mut self.parsed
    }

    fn push_frame(&mut self, frame_type: FrameType, payload: &[u8], rs_error: i32) {
        debug_assert!(rs_error >= 0);

        let mut frame = Frame {
            frame_type,
            payload: Vec::with_capacity(payload.len()),
            rs_error: rs_error as u32,
        };
        frame.payload.extend_from_slice(payload);

        trace!("got a ADS-B frame: {:?}", frame);
        self.parsed.push_back(frame);
    }
}

impl Drop for Dump978 {
    fn drop(&mut self) {
        self.destroy();
    }
}

unsafe impl Send for Dump978 {}

extern "C" fn callback(inst: *mut c_void, frame_type: i32, payload: *const u8, rs_error: i32) {
    let f_type;
    let payload_length;
    let inst = inst as *mut Dump978;

    match frame_type {
        ADS_B_SHORT => {
            f_type = FrameType::ADSBShort;
            payload_length = ADS_B_SHORT_LEN;
        }
        ADS_B_LONG => {
            f_type = FrameType::ADSBLong;
            payload_length = ADS_B_LONG_LEN;
        }
        GROUND_UPLINK => {
            f_type = FrameType::GroundUplink;
            payload_length = GROUND_UPLINK_LEN;
        }
        _ => unreachable!(),
    }

    unsafe {
        (*inst).push_frame(f_type, from_raw_parts(payload, payload_length), rs_error);
    }
}
