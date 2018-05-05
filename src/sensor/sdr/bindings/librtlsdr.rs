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

//! A simple binding for `librtlsdr`
//! Note: this module does not implement all functions exported from `librtlsdr`
//! but only the ones currently needed by pitot.
//!
//! This module requires your system to have `librtlsdr` available for linking.
//! Go to https://github.com/steve-m/librtlsdr if you need to install.

use std::ptr;
use std::io::{self, Read};

enum RtlSDRDevT {}

#[link(name = "rtlsdr")]
extern "C" {
    fn rtlsdr_get_device_count() -> u32;
    fn rtlsdr_get_device_usb_strings(index: u32,
                                     manufact: *mut u8,
                                     product: *mut u8,
                                     serial: *mut u8)
                                     -> i32;
    fn rtlsdr_open(dev: *mut *const RtlSDRDevT, index: u32) -> i32;
    fn rtlsdr_close(dev: *const RtlSDRDevT) -> i32;
    fn rtlsdr_set_tuner_gain_mode(dev: *const RtlSDRDevT, manual: i32) -> i32;
    fn rtlsdr_set_tuner_gain(dev: *const RtlSDRDevT, gain: i32) -> i32;
    fn rtlsdr_set_sample_rate(dev: *const RtlSDRDevT, rate: i32) -> i32;
    fn rtlsdr_set_xtal_freq(dev: *const RtlSDRDevT, rtl_freq: u32, tuner_freq: u32) -> i32;
    fn rtlsdr_set_center_freq(dev: *const RtlSDRDevT, freq: u32) -> i32;
    fn rtlsdr_set_tuner_bandwidth(dev: *const RtlSDRDevT, bw: u32) -> i32;
    fn rtlsdr_reset_buffer(dev: *const RtlSDRDevT) -> i32;
    fn rtlsdr_read_sync(dev: *const RtlSDRDevT, buf: *mut u8, len: i32, n_read: *mut i32) -> i32;
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Unknown,
    Closed,
}

#[derive(Debug, PartialEq)]
/// [`Device`] represents a handle to the underlying SDR device.
pub struct Device {
    dev: *const RtlSDRDevT,
    is_open: bool,
}

#[derive(Debug, PartialEq)]
/// [`HWInfo`] represents information about an underlying SDR device
pub struct HWInfo {
    pub index: u32,
    pub manufact: String,
    pub product: String,
    pub serial: String,
}

impl Device {
    /// Open a new RTL-SDR device with the given index.
    pub fn new(index: u32) -> Result<Self, Error> {
        let mut me = Self {
            dev: ptr::null(),
            is_open: true,
        };

        unsafe {
            if rtlsdr_open(&mut me.dev, index) != 0 {
                Err(Error::Unknown)
            } else {
                Ok(me)
            }
        }
    }

    /// Closes the device. If it is already closed,
    /// this function returns immediately.
    pub fn close(&mut self) -> Result<(), Error> {
        if self.is_open() {
            unsafe {
                if rtlsdr_close(self.dev) != 0 {
                    Err(Error::Unknown)
                } else {
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Returns whether the device is still open.
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn set_tuner_gain_mode(&mut self, enabled: bool) -> Result<&mut Self, Error> {
        if self.is_open() {
            unsafe {
                if rtlsdr_set_tuner_gain_mode(self.dev, enabled as i32) != 0 {
                    Err(Error::Unknown)
                } else {
                    Ok(self)
                }
            }
        } else {
            Err(Error::Closed)
        }
    }

    pub fn set_tuner_gain(&mut self, gain: i32) -> Result<&mut Self, Error> {
        if self.is_open() {
            unsafe {
                if rtlsdr_set_tuner_gain(self.dev, gain) != 0 {
                    Err(Error::Unknown)
                } else {
                    Ok(self)
                }
            }
        } else {
            Err(Error::Closed)
        }
    }

    pub fn set_sample_rate(&mut self, rate: i32) -> Result<&mut Self, Error> {
        if self.is_open() {
            unsafe {
                if rtlsdr_set_sample_rate(self.dev, rate) != 0 {
                    Err(Error::Unknown)
                } else {
                    Ok(self)
                }
            }
        } else {
            Err(Error::Closed)
        }
    }

    pub fn set_xtal_freq(&mut self, rtl_freq: u32, tuner_freq: u32) -> Result<&mut Self, Error> {
        if self.is_open() {
            unsafe {
                if rtlsdr_set_xtal_freq(self.dev, rtl_freq, tuner_freq) != 0 {
                    Err(Error::Unknown)
                } else {
                    Ok(self)
                }
            }
        } else {
            Err(Error::Closed)
        }
    }

    pub fn set_center_freq(&mut self, freq: u32) -> Result<&mut Self, Error> {
        if self.is_open() {
            unsafe {
                if rtlsdr_set_center_freq(self.dev, freq) != 0 {
                    Err(Error::Unknown)
                } else {
                    Ok(self)
                }
            }
        } else {
            Err(Error::Closed)
        }
    }

    pub fn set_tuner_bandwidth(&mut self, bw: u32) -> Result<&mut Self, Error> {
        if self.is_open() {
            unsafe {
                if rtlsdr_set_tuner_bandwidth(self.dev, bw) != 0 {
                    Err(Error::Unknown)
                } else {
                    Ok(self)
                }
            }
        } else {
            Err(Error::Closed)
        }
    }

    pub fn reset_buffer(&mut self) -> Result<&mut Self, Error> {
        if self.is_open() {
            unsafe {
                if rtlsdr_reset_buffer(self.dev) != 0 {
                    Err(Error::Unknown)
                } else {
                    Ok(self)
                }
            }
        } else {
            Err(Error::Closed)
        }
    }
}

impl Read for Device {
    /// Read some bytes from the device with blocking.
    ///
    /// Returns the number of bytes actually read.
    ///
    /// # Errors
    /// * `io::ErrorKind::TimedOut` if there is no more data at this moment
    /// * `io::ErrorKind::Other` some other error occured.
    ///
    /// You can use `into_inner()` to get an error string describing the error code
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut read = 0;

        unsafe {
            match rtlsdr_read_sync(self.dev, buf.as_mut_ptr(), buf.len() as i32, &mut read) {
                0 => {
                    debug_assert!(read >= 0 && read <= buf.len() as i32);
                    Ok(read as usize)
                }
                -7 => {
                    // LIBUSB_ERROR_TIMEOUT
                    Err(io::Error::new(io::ErrorKind::TimedOut,
                                       "no more data to read at this moment"))
                }
                code => {
                    Err(io::Error::new(io::ErrorKind::Other, format!("libusb error: {}", code)))
                }
            }
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

unsafe impl Send for Device {}

/// Get the number of RTL-SDR devices on the current system.
pub fn get_device_count() -> u32 {
    unsafe { rtlsdr_get_device_count() }
}

pub fn get_device_info(index: u32) -> Option<HWInfo> {
    let mut manufact = vec![0; 256];
    let mut product = vec![0; 256];
    let mut serial = vec![0; 256];

    unsafe {
        if rtlsdr_get_device_usb_strings(index,
                                         manufact.as_mut_ptr(),
                                         product.as_mut_ptr(),
                                         serial.as_mut_ptr()) != 0 {
            None
        } else {
            Some(HWInfo {
                     index,
                     manufact: String::from_utf8(manufact).unwrap(),
                     product: String::from_utf8(product).unwrap(),
                     serial: String::from_utf8(serial).unwrap(),
                 })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_device_count() {
        assert!(get_device_count() < 3);
    }

    #[test]
    fn test_new_nx_device() {
        assert_eq!(Device::new(100), Err(Error::Unknown))
    }
}
