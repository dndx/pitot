[![Build Status](https://travis-ci.org/dndx/pitot.svg?branch=master)](https://travis-ci.org/dndx/pitot)
[![Cargo Release](https://img.shields.io/crates/v/pitot.svg)](https://crates.io/crates/pitot)

# Name
Pitot - a customizable aviation information receiver

# Disclaimers
Please note that Pitot is only an aid of situational awareness and should not
be used as the primary source of obtaining flight related information. The
Pilot-in-command always assumes the responsibility of ensuring the safe outcome
of any flight they operate.

The software is still under heavy development and may not be feature complete
and may contain bugs. Use at your own risk.

# Latest release
## pitot-v0.0.1-alpha1-debug.img.zip
**sha256:** `a7151e5f69d6a0b6e567a8a137049cf744e9aacd884187f206c8c85ec73449fa`

**Download Link:** https://github.com/dndx/pitot/releases/tag/v0.0.1-alpha1

All Pitot releases are signed using Datong's PGP key
[CF7004EE981151C8](https://pgp.mit.edu/pks/lookup?op=get&search=0xCF7004EE981151C8).

# Supported
## Motherboard
* Raspberry Pi 3

### Need confirmation
* Older Raspberry Pi models

## EFB
Notice that I currently only have ForeFlight available on my iPad, Pitot
likely supports much more EFB than listed here but not all of them has been
tested.

If you have tested Pitot with your EFB and confirmed it is working, please
open a Pull Request to update this list.

* ForeFlight 9 on iOS ([Screenshot 1](https://user-images.githubusercontent.com/1131072/28495314-7c1fef4c-6efd-11e7-9dd2-3fdf6c10fd02.PNG),
[Screenshot 2](https://user-images.githubusercontent.com/1131072/28495315-7dddd81c-6efd-11e7-9eb8-e3dc514cb8e8.PNG))
* Avare with Avare External IO on Android (confirmed by [@D35Bonanza](https://github.com/D35Bonanza))

## GNSS
* U-blox over GPIO (such as RY83xAI)

### Planned
* **High priority:** U-blox over USB
(should be easy to integrate but disabled for now as I do not have
necessary hardware for testing)
* **Low priority:** Generic GPS with NMEA protocol

## SDR
* Any RTL based SDRs

## Products
* ADS-B, ADS-R and TIS-B traffic (978 UAT and 1080 ES)
* All FIS-B products (978 UAT)

### Planned
* **Medium priority:** AHRS

## Protocol/Transport
* GDL90 over UDP
* WebSocket (still under development)
* GDL90 message buffering when device is sleeping or EFB is not active

### Planned
* **High priority:** Web interface or control App
* **Low priority:** Serial output for EFIS integration

# Recommended hardware build
* [Raspberry Pi 3 Motherboard](https://www.amazon.com/gp/product/B01CD5VC92)
* [SanDisk 16GB Micro SDHC card](https://www.amazon.com/SanDisk-Ultra-Micro-Adapter-SDSQUNC-016G-GN6MA/dp/B010Q57SEE)
* [Stratux low power SDRs and antennas](https://www.amazon.com/gp/product/B01M7NMWCD)
* [6000mAh pass-through charging battery pack](https://www.amazon.com/gp/product/B00ZWUZG70)
* Any casing you like with calling fan ([Stratux case](https://www.amazon.com/gp/product/B072ND582W) works as well)
* [RY836AI WAAS enabled GPS with IMU and baro sensor](https://www.ebay.com/itm/182087516214)

# Getting started
First, program your SD card using the [latest release image](https://github.com/dndx/pitot/releases).

For Mac users, you may use [Pi Filler](http://ivanx.com/raspberrypi/). For Windows users, check out
this [article](https://www.raspberrypi.org/documentation/installation/installing-images/windows.md)
from Raspberry Pi Foundation.

If you need instructions for doing this on Linux, you should probably
consider use something else :)

Next, insert the SD card into your Raspberry Pi's card slot, observe the correct side
with the pins before inserting.

Now, power on your Raspberry Pi and give it up to 3 minutes to finish up initial setup.
You may observe the `Pitot` Wi-Fi showing up but unable to connect, this is normal
when first powered on and DO NOT unplug the power cable until Raspberry Pi has finished
the setup process. If you have lost power before the setup process is completed and you
believe your SD card is bad, simply reprogram the card and try again.

During the setup, your Raspberry Pi will reboot multiple times to expand partitions and
setup the readonly file system. As soon as you are able to connect to the `Pitot` Wi-Fi and
observe data flowing into your EFB, the setup has been completed. If you are unsure, just wait
3 minutes and it will be finished and ready for use.

To power off Pitot simply unplug the power cable, like how you shutdown any avionics inside your
airplane. Pitot uses readonly file system while
running and otherwise does not write to the partition and thus a complete Linux shutdown
is neither expected nor necessary.

Currently Pitot works pretty much out of the box, with minimum configuration required.

For Pitot to use your installed SDRs for 1090 and/or 978 reception, you must program
the serial of the SDRs to contain strings `1090` or `978` respectively.
Pitot will **not** use the SDR at all unless it includes those strings (aka. no guessing).

For GNSS module, Pitot will detect whether your Ublox chip is running the 3.01 firmware
and will enable Galileo constellation tracking automatically.

# Contributing
Please check out [DEVELOPING.md](https://github.com/dndx/pitot/blob/master/DEVELOPING.md)
for guides on how to develop, test, build and contribute to Pitot.

Any improvements are welcomed! However, if you don't have anything specific in mind,
planned features as mentioned above are a good way to get started!

# Integrating
If you want to integrate Pitot with your EFB, please check out
[INTEGRATION.md](https://github.com/dndx/pitot/blob/master/INTEGRATION.md) for more information.

# Helps and discussions
* [Mailing list/forum for general discussions](https://groups.google.com/forum/#!forum/pitot-discussions)
* [Mailing list for announcements](https://groups.google.com/forum/#!forum/pitot-announcements)
* Development discussions - GitHub Issues

We recommend you always subscribe to the announcement mailing list for notifications on new releases.
It is a moderated mailing list and is expected to be extremely low on traffic.

You must join the general discussions forum before you can post in it. There is an option
while joining to optionally receive email notifications from the general discussion forum.
Keep in mind the traffic in the forum might be high and should you choose to receive email,
receiving them in digest is highly recommended.

# Credits
* The [Stratux](https://github.com/cyoung/stratux) project by Christopher Young
for his great work in making the first open source ADS-B receiver and open sourcing
it.
Some algorithms used by Pitot, especially the UAT parsing function were heavily
borrowed from the Stratux project.

# Copyright and License
This project is licensed under the GPLv3 license.

Copyright (C) 2017  Datong Sun (dndx@idndx.com)

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
