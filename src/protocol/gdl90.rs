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
use processor::fisb::FISBData;
use processor::ownship::Ownship;
use processor::traffic::*;
use processor::Report;
use std::time::Instant;
use time::Tm;

const LON_LAT_RESOLUTION: f32 = 180.0 / 8388608.0; // 2^23 (p. 19)
const TRACK_RESOLUTION: f32 = 360.0 / 256.0;
// using Garmin's sample program on page p. 7
const CRC16_TABLE: [u16; 256] = [
    0x0000, 0x1021, 0x2042, 0x3063, 0x4084, 0x50A5, 0x60C6, 0x70E7, 0x8108, 0x9129, 0xA14A, 0xB16B,
    0xC18C, 0xD1AD, 0xE1CE, 0xF1EF, 0x1231, 0x0210, 0x3273, 0x2252, 0x52B5, 0x4294, 0x72F7, 0x62D6,
    0x9339, 0x8318, 0xB37B, 0xA35A, 0xD3BD, 0xC39C, 0xF3FF, 0xE3DE, 0x2462, 0x3443, 0x0420, 0x1401,
    0x64E6, 0x74C7, 0x44A4, 0x5485, 0xA56A, 0xB54B, 0x8528, 0x9509, 0xE5EE, 0xF5CF, 0xC5AC, 0xD58D,
    0x3653, 0x2672, 0x1611, 0x0630, 0x76D7, 0x66F6, 0x5695, 0x46B4, 0xB75B, 0xA77A, 0x9719, 0x8738,
    0xF7DF, 0xE7FE, 0xD79D, 0xC7BC, 0x48C4, 0x58E5, 0x6886, 0x78A7, 0x0840, 0x1861, 0x2802, 0x3823,
    0xC9CC, 0xD9ED, 0xE98E, 0xF9AF, 0x8948, 0x9969, 0xA90A, 0xB92B, 0x5AF5, 0x4AD4, 0x7AB7, 0x6A96,
    0x1A71, 0x0A50, 0x3A33, 0x2A12, 0xDBFD, 0xCBDC, 0xFBBF, 0xEB9E, 0x9B79, 0x8B58, 0xBB3B, 0xAB1A,
    0x6CA6, 0x7C87, 0x4CE4, 0x5CC5, 0x2C22, 0x3C03, 0x0C60, 0x1C41, 0xEDAE, 0xFD8F, 0xCDEC, 0xDDCD,
    0xAD2A, 0xBD0B, 0x8D68, 0x9D49, 0x7E97, 0x6EB6, 0x5ED5, 0x4EF4, 0x3E13, 0x2E32, 0x1E51, 0x0E70,
    0xFF9F, 0xEFBE, 0xDFDD, 0xCFFC, 0xBF1B, 0xAF3A, 0x9F59, 0x8F78, 0x9188, 0x81A9, 0xB1CA, 0xA1EB,
    0xD10C, 0xC12D, 0xF14E, 0xE16F, 0x1080, 0x00A1, 0x30C2, 0x20E3, 0x5004, 0x4025, 0x7046, 0x6067,
    0x83B9, 0x9398, 0xA3FB, 0xB3DA, 0xC33D, 0xD31C, 0xE37F, 0xF35E, 0x02B1, 0x1290, 0x22F3, 0x32D2,
    0x4235, 0x5214, 0x6277, 0x7256, 0xB5EA, 0xA5CB, 0x95A8, 0x8589, 0xF56E, 0xE54F, 0xD52C, 0xC50D,
    0x34E2, 0x24C3, 0x14A0, 0x0481, 0x7466, 0x6447, 0x5424, 0x4405, 0xA7DB, 0xB7FA, 0x8799, 0x97B8,
    0xE75F, 0xF77E, 0xC71D, 0xD73C, 0x26D3, 0x36F2, 0x0691, 0x16B0, 0x6657, 0x7676, 0x4615, 0x5634,
    0xD94C, 0xC96D, 0xF90E, 0xE92F, 0x99C8, 0x89E9, 0xB98A, 0xA9AB, 0x5844, 0x4865, 0x7806, 0x6827,
    0x18C0, 0x08E1, 0x3882, 0x28A3, 0xCB7D, 0xDB5C, 0xEB3F, 0xFB1E, 0x8BF9, 0x9BD8, 0xABBB, 0xBB9A,
    0x4A75, 0x5A54, 0x6A37, 0x7A16, 0x0AF1, 0x1AD0, 0x2AB3, 0x3A92, 0xFD2E, 0xED0F, 0xDD6C, 0xCD4D,
    0xBDAA, 0xAD8B, 0x9DE8, 0x8DC9, 0x7C26, 0x6C07, 0x5C64, 0x4C45, 0x3CA2, 0x2C83, 0x1CE0, 0x0CC1,
    0xEF1F, 0xFF3E, 0xCF5D, 0xDF7C, 0xAF9B, 0xBFBA, 0x8FD9, 0x9FF8, 0x6E17, 0x7E36, 0x4E55, 0x5E74,
    0x2E93, 0x3EB2, 0x0ED1, 0x1EF0,
];
const HEARTBEAT_FREQ: u16 = 1;
const OWNSHIP_FREQ: u16 = 2;
const MAX_STALE_SECS: u64 = 6; // do not report data more than 6 sec old

pub struct GDL90 {
    ownship_valid: bool,
    heartbeat_counter: u32,
    ownship_counter: u32,
    /// true if Pressure altitude source exists
    pres_alt_valid: bool,
}

impl Protocol for GDL90 {
    fn run(&mut self, handle: &mut Pushable<Payload>, i: ChainedIter) {
        let clock = handle.get_clock();

        self.ownship_counter += 1;
        self.heartbeat_counter += 1;

        for e in i {
            match *e {
                Report::Ownship(ref o) => {
                    if self.ownship_counter >= (handle.get_frequency() / OWNSHIP_FREQ) as u32 {
                        self.ownship_counter = 0;
                        self.ownship_valid = o.valid;

                        if o.pressure_altitude.is_some() {
                            self.pres_alt_valid = true;
                        }

                        handle.push_data(GDL90::generate_ownship(o));
                        handle.push_data(GDL90::generate_ownship_geometric_altitude(o));
                    }
                }
                Report::Traffic(ref o) => {
                    // throttle for Target type is done at traffic processor
                    handle.push_data(GDL90::generate_traffic(o, clock, self.pres_alt_valid));
                }
                Report::FISB(ref o) => handle.push_data(GDL90::generate_uplink(o)),
                _ => {}
            }
        }

        if self.heartbeat_counter == (handle.get_frequency() / HEARTBEAT_FREQ) as u32 {
            self.heartbeat_counter = 0;
            let utc = handle.get_utc();
            handle.push_data(self.generate_heartbeat(&utc));
            handle.push_data(GDL90::generate_foreflight_id());
        }
    }
}

impl GDL90 {
    fn generate_heartbeat(&self, utc: &Tm) -> Payload {
        let mut buf = [0_u8; 7 + 2]; // incl CRC field

        buf[0] = 0x00; // type = heartbeat
        buf[1] = 0x11; // UAT Initialized + ATC Services talkback

        if self.ownship_valid {
            buf[1] |= 0x80;
        }

        let midnight_utc = Tm {
            tm_hour: 0,
            tm_min: 0,
            tm_sec: 0,
            ..*utc
        };
        let delta = (*utc - midnight_utc).num_seconds();

        buf[2] = ((delta & 0x10000) >> 9) as u8 | 0x01; // MSB + UTC OK
        buf[3] = (delta & 0xFF) as u8;
        buf[4] = ((delta & 0xFF00) >> 8) as u8;

        Payload {
            queueable: false,
            payload: GDL90::prepare_payload(&mut buf),
        }
    }

    fn generate_foreflight_id() -> Payload {
        // see: https://www.foreflight.com/connect/spec/

        let mut buf = [0_u8; 39 + 2]; // incl CRC field

        buf[0] = 0x65; // type = FF
        buf[1] = 0x00; // sub ID = 0
        buf[2] = 0x01; // version = 1

        for i in 3..11 {
            buf[i] = 0xFF; // serial = invalid
        }

        buf[11] = 'P' as u8;
        buf[12] = 'i' as u8;
        buf[13] = 't' as u8;
        buf[14] = 'o' as u8;
        buf[15] = 't' as u8;

        buf[20] = 'P' as u8;
        buf[21] = 'i' as u8;
        buf[22] = 't' as u8;
        buf[23] = 'o' as u8;
        buf[24] = 't' as u8;

        buf[38] = 0x01; // geometric altitude datum = MSL

        Payload {
            queueable: false,
            payload: GDL90::prepare_payload(&mut buf),
        }
    }

    fn generate_uplink(e: &FISBData) -> Payload {
        let mut buf = [0_u8; 436 + 2]; // incl CRC field

        buf[0] = 0x07; // type = uplink

        buf[1] = 0xFF;
        buf[2] = 0xFF;
        buf[3] = 0xFF;

        &buf[4..436].clone_from_slice(&e.payload);

        Payload {
            queueable: true,
            payload: GDL90::prepare_payload(&mut buf),
        }
    }

    fn generate_ownship_geometric_altitude(e: &Ownship) -> Payload {
        let mut buf = [0_u8; 5 + 2]; // incl CRC field

        buf[0] = 0x0B; // type = ownship geometric

        let alt = (e.altitude / 5) as i16;

        buf[1] = (alt >> 8) as u8;
        buf[2] = (alt & 0x00FF) as u8;

        buf[3] = 0x00;
        buf[4] = 0x0A; // No Vertical Warning, VFOM = 10 meters

        Payload {
            queueable: false,
            payload: GDL90::prepare_payload(&mut buf),
        }
    }

    fn generate_ownship(e: &Ownship) -> Payload {
        let mut buf = [0_u8; 28 + 2]; // incl CRC field

        buf[0] = 0x0A;
        buf[1] = 0x01; // alert status = false, identity = ADS-B with Self-assigned address
        buf[2] = 0xF0; // self-assigned address
        buf[3] = 0x00;
        buf[4] = 0x00;

        // latitude
        let (lat1, lat2, lat3) = latlon_to_gdl90(e.lat);
        buf[5] = lat1;
        buf[6] = lat2;
        buf[7] = lat3;

        // longitude
        let (lon1, lon2, lon3) = latlon_to_gdl90(e.lon);
        buf[8] = lon1;
        buf[9] = lon2;
        buf[10] = lon3;

        // altitude
        if let Some(alt) = e.pressure_altitude {
            let alt = alt_to_gdl90(alt as f32);
            buf[11] = ((alt & 0xFF0) >> 4) as u8;
            buf[12] = (((alt & 0x00F) << 4) | 0x09) as u8; // Airborne + True Track
        } else {
            buf[11] = 0xFF;
            buf[12] = 0xF9; // Airborne + True Track
        }

        buf[13] = (e.nic << 4) & 0xF0 | e.nacp & 0x0F;

        let gs = e.gs.round() as u16;
        let vs = 0x800_u16; // "no vertical rate available"
        buf[14] = ((gs & 0xFF0) >> 4) as u8;
        buf[15] = (((gs & 0x00F) << 4) | ((vs & 0x0F00) >> 8)) as u8;
        buf[16] = (vs & 0xFF) as u8;

        buf[17] = crs_to_gdl90(e.track);

        buf[18] = 0x01; // Light (ICAO) < 15 500 lbs

        buf[19] = 'P' as u8;
        buf[20] = 'i' as u8;
        buf[21] = 't' as u8;
        buf[22] = 'o' as u8;
        buf[23] = 't' as u8;

        Payload {
            queueable: false,
            payload: GDL90::prepare_payload(&mut buf),
        }
    }

    fn generate_traffic(e: &Target, clock: Instant, pres_alt_valid: bool) -> Payload {
        let mut buf = [0_u8; 28 + 2]; // incl CRC field

        buf[0] = 0x14;

        buf[1] = match e.addr.1 {
            AddressType::ADSBICAO | AddressType::ADSRICAO => 0,
            AddressType::ADSBOther | AddressType::ADSROther => 1,
            AddressType::TISBICAO => 2,
            AddressType::TISBOther => 3,
            _ => 3, // unknown
        };

        buf[2] = ((0xFF0000 & e.addr.0) >> 16) as u8; // address
        buf[3] = ((0x00FF00 & e.addr.0) >> 8) as u8;
        buf[4] = (0x0000FF & e.addr.0) as u8;

        // latitude
        if let Some(((lat, lon), i)) = e.lat_lon {
            if (clock - i).as_secs() <= MAX_STALE_SECS {
                let (lat1, lat2, lat3) = latlon_to_gdl90(lat);
                buf[5] = lat1;
                buf[6] = lat2;
                buf[7] = lat3;

                // longitude
                let (lon1, lon2, lon3) = latlon_to_gdl90(lon);
                buf[8] = lon1;
                buf[9] = lon2;
                buf[10] = lon3;

                if let Some(nic) = e.nic {
                    buf[13] |= (nic << 4) & 0xF0;
                }
            }
        }

        // altitude
        if let Some((alt, typ, i)) = e.altitude {
            if (clock - i).as_secs() <= MAX_STALE_SECS {
                let mut corrected_alt = alt;

                // if ownship pressure altitude is NOT available, use MSL and attempt to correct it
                // using GNSS delta if needed
                if !pres_alt_valid && typ == AltitudeType::Baro {
                    // GDL90 wants pres altitude, try to calculate it from GNSS altitude
                    // if correction is available

                    // Note: GDL90 wants pressure altitude here,
                    // but FF currently uses MSL altitude from
                    // ownship geometric report when calculating altitude
                    // difference, this is to correct Baro altitude
                    // to MSL so that the calculation will be as accurate as possible
                    if let Some(delta) = e.gnss_delta {
                        corrected_alt += delta;
                    }
                } else if pres_alt_valid && typ == AltitudeType::GNSS {
                    if let Some(delta) = e.gnss_delta {
                        corrected_alt -= delta;
                    }
                }

                let alt = alt_to_gdl90(corrected_alt as f32);
                buf[11] = ((alt & 0xFF0) >> 4) as u8;
                buf[12] = ((alt & 0x00F) << 4) as u8;
            }
        } else {
            // invalid altitude
            buf[11] = 0xFF;
            buf[12] = 0xF0;
        }

        if let Some((_, typ, i)) = e.heading {
            if (clock - i).as_secs() <= MAX_STALE_SECS {
                match typ {
                    HeadingType::True => buf[12] |= 0x01,
                    HeadingType::Mag => buf[12] |= 0x02,
                }
            }
        }

        if e.on_ground != Some(true) {
            buf[12] |= 0x08; // airborne
                             // if unknown, assume airborne
        }

        if let Some(nacp) = e.nacp {
            buf[13] |= nacp & 0x0F;
        }

        // velocity unavailable by default
        buf[14] = 0xFF;
        buf[15] = 0xF0;

        if let Some((spd, _, i)) = e.speed {
            if (clock - i).as_secs() <= MAX_STALE_SECS {
                buf[14] = ((spd & 0xFF0) >> 4) as u8;
                buf[15] = ((spd & 0x00F) << 4) as u8;
            }
        }

        if let Some((vs, i)) = e.vs {
            if (clock - i).as_secs() <= MAX_STALE_SECS {
                let vs = (vs as f32 / 64_f32).round() as i16; // see p. 21
                buf[15] |= ((vs & 0xF00) >> 8) as u8;
                buf[16] = (vs & 0xFF) as u8;
            } else {
                buf[15] |= 0x08; // no vs
            }
        } else {
            buf[15] |= 0x08; // no vs
        }

        if let Some((hdg, _, _)) = e.heading {
            // valid flag set above
            buf[17] = crs_to_gdl90(hdg as f32);
        }

        if let Some(cat) = e.category {
            buf[18] = cat;
        }

        // insert traffic source
        buf[19] = match e.source {
            TrafficSource::UAT => 'u',
            TrafficSource::ES => 'e',
        } as u8;

        buf[20] = match e.addr.1 {
            AddressType::ADSBICAO | AddressType::ADSBOther => 'a',
            AddressType::ADSRICAO | AddressType::ADSROther => 'r',
            AddressType::TISBICAO | AddressType::TISBOther => 't',
            _ => 'x',
        } as u8;

        if let Some(ref cs) = e.callsign {
            for (i, c) in cs.chars().take(6).enumerate() {
                buf[21 + i] = c as u8;
            }
        } else if let Some(sq) = e.squawk {
            // squawk available?
            let squawk_str = format!("{:04}", sq); // 0 padded
            debug_assert!(squawk_str.len() == 4);
            let squawk_str = squawk_str.as_bytes();

            buf[21] = squawk_str[0];
            buf[22] = squawk_str[1];
            buf[23] = squawk_str[2];
            buf[24] = squawk_str[3];
        }

        if let Some(sq) = e.squawk {
            if sq == 7700 || sq == 7600 || sq == 7500 {
                buf[27] = 0x10; // emergency aircraft
            }
        }

        Payload {
            queueable: false,
            payload: GDL90::prepare_payload(&mut buf),
        }
    }

    /// Given a buffer containing everything between "Flag Bytes" (see p. 5)
    /// with the CRC field space allocated but left empty for calculation
    fn prepare_payload(buf: &mut [u8]) -> Vec<u8> {
        let len = buf.len() - 2;

        let crc = buf.iter()
            .take(len)
            .scan(0_u16, |crc, b| {
                *crc = CRC16_TABLE[(*crc >> 8) as usize] ^ (*crc << 8) ^ (*b as u16);
                Some(*crc)
            })
            .last()
            .unwrap();

        buf[len] = (crc & 0xFF) as u8;
        buf[len + 1] = (crc >> 8) as u8;

        // len + CRC (2 bytes) + 2 Flag Bytes + some stuffing bits (don't know yet)
        let mut tmp = Vec::with_capacity(len + 4);
        tmp.push(0x7E);

        for b in buf {
            if *b == 0x7E || *b == 0x7D {
                tmp.push(0x7D);
                tmp.push(*b ^ 0x20);
            } else {
                tmp.push(*b);
            }
        }

        tmp.push(0x7E);

        tmp
    }
}

impl GDL90 {
    pub fn new() -> Box<Protocol> {
        Box::new(GDL90 {
            ownship_valid: false,
            heartbeat_counter: 0,
            ownship_counter: 0,
            pres_alt_valid: false,
        })
    }
}

/// Given coordinate in degrees, return the GDL 90 formatted byte sequence
/// From: https://github.com/cyoung/stratux/blob/master/main/gen_gdl90.go#L206
fn latlon_to_gdl90(mut d: f32) -> (u8, u8, u8) {
    d /= LON_LAT_RESOLUTION;
    let wk = d.round() as i32;

    (
        ((wk & 0xFF0000) >> 16) as u8,
        ((wk & 0x00FF00) >> 8) as u8,
        (wk & 0x0000FF) as u8,
    )
}

fn alt_to_gdl90(mut a: f32) -> u16 {
    if a < -1000_f32 || a > 101350_f32 {
        0xFFF
    } else {
        a += 1000_f32; // see p. 20
        a /= 25_f32;

        (a.round() as u16) & 0xFFF
    }
}

fn crs_to_gdl90(mut c: f32) -> u8 {
    while c > 360_f32 {
        c -= 360_f32;
    }

    while c < 0_f32 {
        c += 360_f32;
    }

    (c / TRACK_RESOLUTION) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alt_to_gdl90() {
        assert_eq!(alt_to_gdl90(-2000_f32), 0xFFF);
        assert_eq!(alt_to_gdl90(-1001_f32), 0xFFF);
        assert_eq!(alt_to_gdl90(-1000_f32), 0x000);
        assert_eq!(alt_to_gdl90(-975_f32), 0x001);
        assert_eq!(alt_to_gdl90(0_f32), 0x028);
        assert_eq!(alt_to_gdl90(1000_f32), 0x050);
        assert_eq!(alt_to_gdl90(1001_f32), 0x050);
        assert_eq!(alt_to_gdl90(1025_f32), 0x051);
        assert_eq!(alt_to_gdl90(101350_f32), 0xFFE);
        assert_eq!(alt_to_gdl90(101351_f32), 0xFFF);
    }

    #[test]
    fn test_crs_to_gdl90() {
        assert_eq!(crs_to_gdl90(0_f32), 0x00);
        assert_eq!(crs_to_gdl90(180_f32), 0x80);
        assert_eq!(crs_to_gdl90(266_f32), 0xBD);
        assert_eq!(crs_to_gdl90(359_f32), 0xFF);
        assert_eq!(crs_to_gdl90(360_f32), 0x00);
    }

    #[test]
    fn test_generate_traffic() {
        let clock = Instant::now();
        let mut trfc = Target::new(
            (0xA1B2C3, AddressType::ADSBICAO),
            clock,
            TrafficSource::ES,
            None,
        );

        trfc.altitude = Some((12375, AltitudeType::Baro, clock));
        trfc.gnss_delta = Some(1000);
        trfc.heading = Some((123, HeadingType::True, clock));
        trfc.speed = Some((66, SpeedType::GS, clock));
        trfc.vs = Some((-1000, clock));
        trfc.squawk = Some(123);
        trfc.callsign = Some("TEST123".into());
        trfc.category = Some(3);
        trfc.lat_lon = Some(((37.750374, -122.52676), clock));
        trfc.nic = Some(7);
        trfc.nacp = Some(9);
        trfc.on_ground = Some(false);

        let payload = GDL90::generate_traffic(&trfc, clock, false);
        let expected = [
            0x7E, 0x14, 0x00, 0xA1, 0xB2, 0xC3, 0x1A, 0xD8, 0x3F, 0xA8, 0xDE, 0xAF, 0x23, 0xF9,
            0x79, 0x04, 0x2F, 0xF0, 0x57, 0x03, 'e' as u8, 'a' as u8, 'T' as u8, 'E' as u8,
            'S' as u8, 'T' as u8, '1' as u8, '2' as u8, 0x00, 0x4D, 0xDE, 0x7E,
        ];

        assert_eq!(payload.payload, &expected);

        let payload = GDL90::generate_traffic(&trfc, clock, true);
        let expected = [
            0x7E, 0x14, 0x00, 0xA1, 0xB2, 0xC3, 0x1A, 0xD8, 0x3F, 0xA8, 0xDE, 0xAF, 0x21, 0x79,
            0x79, 0x04, 0x2F, 0xF0, 0x57, 0x03, 'e' as u8, 'a' as u8, 'T' as u8, 'E' as u8,
            'S' as u8, 'T' as u8, '1' as u8, '2' as u8, 0x00, 0xEA, 0xC4, 0x7E,
        ];

        assert_eq!(payload.payload, &expected);

        trfc.callsign = None;
        let payload = GDL90::generate_traffic(&trfc, clock, false);
        let expected = [
            0x7E, 0x14, 0x00, 0xA1, 0xB2, 0xC3, 0x1A, 0xD8, 0x3F, 0xA8, 0xDE, 0xAF, 0x23, 0xF9,
            0x79, 0x04, 0x2F, 0xF0, 0x57, 0x03, 'e' as u8, 'a' as u8, '0' as u8, '1' as u8,
            '2' as u8, '3' as u8, 0x00, 0x00, 0x00, 0x87, 0xEC, 0x7E,
        ];

        assert_eq!(payload.payload, &expected);

        trfc.altitude = Some((12375, AltitudeType::GNSS, clock));
        let payload = GDL90::generate_traffic(&trfc, clock, true);
        let expected = [
            0x7E, 0x14, 0x00, 0xA1, 0xB2, 0xC3, 0x1A, 0xD8, 0x3F, 0xA8, 0xDE, 0xAF, 0x1E, 0xF9,
            0x79, 0x04, 0x2F, 0xF0, 0x57, 0x03, 'e' as u8, 'a' as u8, '0' as u8, '1' as u8,
            '2' as u8, '3' as u8, 0x00, 0x00, 0x00, 0x12, 0x2D, 0x7E,
        ];

        assert_eq!(payload.payload, &expected);
    }
}
