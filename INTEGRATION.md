# Guides for system integrators
This document intends to explain how Pitot can be integrated into your own product.

# GDL 90
GDL 90 protocol is currently the recommend way of utilizing Pitot generated information.
Pitot actively monitors clients that has an active DHCP lease on the Wi-Fi network and
will send unicast UDP packets containing GDL 90 paylaods to each client on port `4000`.

## Packet structure
Pitot optimizes for sending minimum amount of UDP packet for reduced battery consumption,
and will attempt to squeeze as much GDL 90 message as possible into a UDP packet without
causing IP fragmentation. Each message has the flag byte, message id, message data, checksum
and flag byte as specified in the GDL 90 protocol specification. Byte stuffing are used
when necessary as specified.

Each UDP packet will contain one or more GDL 90 message, without any extra bytes in between.
Observe the normal GDL 90 framing/stuffing rules while parsing.

GDL 90 messages sent by Pitot will never cross packet boundary. If adding a new GDL 90
message will cause the current UDP datagram to fragment, Pitot will create a new
UDP datagram instead.

There is no particular order on how messages are being sent (and in general UDP does not
guarantee strong ordering anyway). You should not rely
on the ordering of message while processing Pitot's input.

## Messages Pitot generates
| Message                       | Frequency    |
| ----------------------------- | ------------ |
| Heartbeat                     | 1 Hz         |
| Ownship                       | 2 Hz         |
| Ownship Geometric Altitude    | 2 Hz         |
| Traffic                       | 1 Hz         |
| Uplink Data (FIS-B)           | As available |

## Discrepancies between Pitot and GDL 90 (and Stratux)
GDL 90 was designed for panel installed ADS-B receiver, and thus have a very good source of
pressure altitude. Because of that, most of the GDL 90 message deals with pressure altitude.
Pitot is designed as a portable ADS-B receiver running inside the cabin, and there is no
reliable pressure altitude source available. With the fan running inside the case and
pressurized cabin, the situation gets even more complicated.

For Ownship Geometric message, GDL 90 asks for height above ellipsoid, but it appears to me that
no EFB uses height above ellipsoid, and most of them just treats it as MSL altitude.

Because of this, Pitot choose to use MSL altitude for all messages it generates. This includes
Ownship and Ownship Geometric Altitude. For traffic message, when Pitot can derive the MSL
altitude for traffic that squawks out GNSS delta information, it will correct pressure altitude
using that delta and provide MSL altitude in the traffic message. If not, pressure altitude
will be outputted instead.

According to my prior flights with Pitot, this is not a huge problem as GNSS delta is generally
very small unless inside the flight levels, and most of the airplane flying that high has 1090 ES
transponder and thus outputs the GNSS delta. Most of the airplanes that do not output GNSS delta
are flying low on the altitude where the delta will not be significant anyway.

I have also observed that Pitot seems to produce more accurate traffic sepration
information than Stratux due to this decision.

# Sleep and inactive detection
Pitot will attempt to detect a client that is sleeping or not actively using the EFB app. If the
client later become active again, the last 8192 FIS-B messages will be replayed to help the client
catch up.

Here is how it works:

Every second, Pitot will send out ICMP Echo Request to all known client. If no ICMP Echo Reply
has been received from a client for more than 3 seconds, that client is considered as *sleeping*.
This is the sleep detection.

Pitot always send all traffic and new FIS-B updates to all clients regardless of their state.
If "Connection refused" error was detected, that client is considered as *not in app*.
If no "Connection refused" was seen in the last 5 seconds, the client is considered as back *in app*.

When transition from *sleeping* to *not sleeping* occurs, Pitot marks the client as *not in app*
and resets the in app detection timer to present.

When transition from *not in app* to *in app* occurs, the buffered FIS-B messages will be replayed.

**Note:**
1. The replay can only occur at most once every 30 seconds. This is to prevent a flapping device
from consuming too much resources.
2. Real time traffic and FIS-B updates are always being send to all clients regardless
if their *in app* and *sleeping* states. This is to ensure that when switching into the App the latest
information are always available immediately and to facilitate the *in app* detection.

This design will work well with the following cases:
* Client left the App open but turn off the screen.
* Client left the App to another App or the SpringBoard.
* Client left the App to another App or the SpringBoard and turned off the screen.

# Problems
If you have any questions while integrating Pitot, feel free to open a GitHub Issue
and I will try my best to help.
