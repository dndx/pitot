# Pitot Development Guide
Thanks for your interest in making Pitot better! Here are some useful information that will
help you get started in developing for Pitot.

# Language
The majority of Pitot's logic has been written in [Rust](https://www.rust-lang.org), and chances are you need to learn some
amount of Rust to be able to contribute to Pitot.

Unfortunately Rust is not known for having the shallowest learning curve, meaning you may need to spend some time learning
before being able to understand all of Pitot's code. However, there are no shortage of good resources on helping you
achieving that. Here are some of my favorites:

* [Rust by Example](http://rustbyexample.com/). This can get you started with Rust relatively quickly.
* [The Rust Programming Language](https://doc.rust-lang.org/book/). You may need to refer to this book often while writing Rust code.
* [Official Rust Documentations Collection](https://www.rust-lang.org/en-US/documentation.html). Including links to the above two.

# Architecture
Very much unlike Stratux, Pitot uses a modular design that intends to abstracts away the knowledge of other components
running in the system. As a result, you can replace/remove any components inside Pitot without causing trouble elsewhere.
This is fundamental for supporting more hardware/protocol/feature as the project grows bigger.

![Pitot architecture drawing](https://user-images.githubusercontent.com/1131072/27773110-302f98ac-5f27-11e7-8a53-baedfa5c694f.png)

Pitot primarily has four stages, and higher stage uses information from lower stage to complete it's tasks (just like a turbine engine).
From bottom to top, we have Sensor stage, Processor stage, Protocol stage and finally, Transport stage. I will explain what each of those
stages does below.

## Sensor stage
Sensor stage is how Pitot acquires data from the outside world. SDR interface, GNSS interface and IMU/baro interface should all be
categorized as sensor component and should be running inside this stage.

Sensor stage takes the raw sensor reading and generates `pitot::sensor::SensorData`, which is an `enum` of multiple possible
parsed sensor reading. `SensorData` will get passed to the Processor stage for further processing.

## Processor stage
Processor stage is how Pitot analyzes sensor input and maintain/generates state information about the world around it.
Processors read `SensorData`, find out the ones they are interested in, and optionally update their internal state
as necessary.

Processor stage will output `pitot::processor::Report` `enum` using their internal state either periodically or under
some determined condition. `Report` will get passed to the Protocol stage for further processing.

## Protocol stage
Protocol stage is how Pitot converts `Report` into output protocol format, for example, GDL90 and JSON. Protocol
stage generally does not have states (although it is possible to).

Protocol stage will output `pitot::protocol::Payload` `struct`. `Payload` will get passed to the Transport stage
for final processing.

## Transport stage
Transport stage is how Pitot outputs the `Payload` from Protocol stage using output interface. For example, UDP
or serial.

Transport stage may have state information. For example, a UDP transport need to maintain the list of all
clients it needs to send to.

Transport stage does not generate any more data in the Pitot processing pipeline as it is the last stage
of the process.

## Main loop
Main loop is what keeps everything in Pitot running. It runs at a fixed frequency (10 Hz at this moment,
but do not hardcode this in your code!) and went through all modules in all stages from lowest stage
to highest stage in a single thread.

Main loop also facilitates message passing from lower stage to higher stage.

## Blocking code
It is critical that your module does NOT use any blocking operation while running inside the main loop.
Doing so will significantly downgrade the performance of Pitot and will cause Pitot to output warning
messages in log.

If you need to run blocking operation, you should run them in a separate thread and passes messages
back to the main loop in an asynchronous manner. See the UAT/1090 ES code for an example of this in action.

## Note
For each module, it will receive all messages pushed by the stage directly below it. Modules uses the
pattern matching feature of Rust to filter out the ones it's interested in and ignore all others.

The current Pitot architecture has been working well for me, although I did had to redesign it at some point
to make it more generalized. That being said, there is still a small possibility the architecture may change,
if we found new drawbacks about it.

# Getting started
Ok, now you have gained some basic understanding on how Pitot operates, let's get started on developing!

## Getting `libdump978` and `libdump1090`
**Note:** despite being the same name, the `libdump978` Pitot uses is NOT the same thing as `libdump978`
Stratux uses. It runs more efficiently by avoiding copying of raw I/Q samples from Pitot to
the library and also have a very different API interface.

**Dependencies:** you need `libusb-dev` and `librtlsdr-dev` installed for those build below to succeed.

```shell
$ git clone git@github.com:dndx/dump978.git
$ cd dump978
$ git checkout libdump978
$ make
$ sudo make install
```

```shell
$ git clone git@github.com:dndx/dump1090.git
$ cd dump1090
$ git checkout libdump1090
$ make
$ sudo make install
```

After that, make sure your system recognizes the newly installed shared libraries by running:
```shell
$ sudo ldconfig
```
and you should be good to go!

Make sure to have the appropriate shared library installed before attempting to build Pitot
as `rustc` will attempt to dynamically link against those libraries and you may got linker
error if they can not be found.

## Setting up Rust environment
Luckily, Rust is pretty easy to setup, simply go to the [Install Rust](https://www.rust-lang.org/en-US/install.html)
page and follow the instructions, you should be up and running in minutes.

## Building Pitot
Building Pitot is extremely simple! Once you have cloned the repository, simply run
```shell
$ cargo build
```

and this will build Rust in debug mode. The binary file will be located under `pitot/target/debug/pitot`.

If you want to build Pitot in release mode instead, do:

```shell
$ cargo build --release
```

The binary file will be located under `pitot/target/release/pitot`.

### Debug vs release
Debug mode turns off all compile time optimizations and turns on a lot of assertions which will make
Pitot run slower. However, during the early stage of development, I recommend you use
debug mode to help catching bugs and make Pitot better!

## Running tests
Pitot has a test set that is still improving. We encourage you run Pitot on every change you make and
of course to contribute tests for existing features and features you have developed. Having a good
test is how we will ensure the consistent quality of Pitot as the project goes forward.

If you have noticed and test failure on your machine, please open an issue with the output and we will
be sure to take a look.

```shell
$ cargo test
```

## Debugging Pitot
Pitot can be debugged using GDB, Valgrind and the built in debug logs. To see the logs, run debug
Pitot manually like this:

```shell
$ RUST_BACKTRACE=1 RUST_LOG=trace path/to/pitot
```

The `RUST_LOG` parameter takes values from `trace` all the way up to `error`. Check out the
documentation of [`env_logger`](https://doc.rust-lang.org/log/env_logger/) for more information.

## Running Pitot on your build
Pitot can run it's tests perfectly fine on x86 machines, but for it to actually work in the cockpit,
you need to make it run on your actual Pitot build. Here is how you do it:

First, build an ARM Pitot binary using either an Raspberry Pi directly or cross-compiling.
I personally have a separate Raspberry Pi hooked on my home network for this purpose.
It is generally easier to setup than cross-compiling.

Next, SSH into Pitot
```shell
$ ssh pi@192.168.0.1
# password is "raspberry", no quotes
```

You can verify that readonly protection is active by running:
```shell
$ df -h
```

and observe `/dev/mmcblk0p2` is mounted as `/mnt/root-ro`.

Now, disable readonly protection on the root partition by editing
`/boot/cmdline.txt` and append `disable-root-ro=true` to the last
and reboot.

If you run `df -h` again, `/dev/mmcblk0p2` will no longer be mounted as
`/mnt/root-ro` and you can write to your root partition now.

Stop the Pitot process by running:
```shell
$ sudo systemctl stop pitot
```

and copy the new binary to overwrite `/usr/local/bin/pitot`.

Next, very important, you need to give Pitot capability to change system time:

```shell
$ sudo setcap CAP_SYS_TIME+ep /usr/local/bin/pitot
```

Once you are done, remove the addition from `/boot/cmdline.txt` and reboot.

Finally, verify your root partition is mounted readonly again by running
`df -h` and check for the output.

# Problems
If you need any help while developing, feel free to open a GitHub Issue and
I will try my best to take a look.

# Useful links
## Pitot
* [libdump978](https://github.com/dndx/dump978/tree/libdump978)
* [libdump1090](https://github.com/dndx/dump1090/tree/libdump1090)
* [Image building script](https://github.com/dndx/rpi-img-builder/tree/pitot)

## 1090 ES
* [ADS-B Decoding Guide](https://adsb-decode-guide.readthedocs.io/en/latest/)
* [Technical Provisions for Mode S Services and Extended Squitter](http://www.cats.com.kh/download.php?path=vdzw4dHS08mjtKi6vNi31Mbn0tnZ2eycn6ydmqPE19rT7Mze4cSYpsetmdXd0w==)
* [ICAO Annex 10 Volume IV Surveillance and Collision Avoidance Systems](https://www.bazl.admin.ch/dam/bazl/de/dokumente/Fachleute/Regulationen_und_Grundlagen/111/icao_annex_10_aeronauticaltelecommunicationsvolumeiv-surveillanc.pdf.download.pdf/icao_annex_10_aeronauticaltelecommunicationsvolumeiv-surveillanc.pdf)

## 978 UAT
* [GDL 90 Protocol Specification](https://www.faa.gov/nextgen/programs/adsb/Archival/media/GDL90_Public_ICD_RevA.PDF)
* [Manual for the Universal Access Transceiver (UAT)](https://www.icao.int/safety/acp/Inactive%20working%20groups%20library/ACP-WG-C-UAT-2/UAT-SWG02-WP04%20-%20Draft%20Tech%20Manual%20V0-1%20.pdf)
* [Surveillance and Broadcast Services Description](https://github.com/cyoung/stratux/raw/master/notes/SBS-Description-Doc_SRT_47_rev01_20111024.pdf)
