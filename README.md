# rumbac - a simple flasher for _Arduino Nano 33 BLE / Sense_

`rumbac` is a simple commandline tool for uploading .bin files with compiled
programs to the _Arduino Nano 33 BLE_ or _... Sense_ board
(with the _nRF52840_ microcontroller).

The boards come preprogrammed with a builtin bootloader,
communicating over a serial port over USB.
The protocol used for that is a very simple, text-based one, called "SAM-BA".
`rumbac` implements a very minimal subset of that protocol,
just enough to allow flashing a .bin file to one of those supported boards.
The implementation is based on a popular `bossac` tool,
in a variant forked by the Arduino team for the _Nano_ board.

To install the tool, run:

    cargo install --git https://github.com/akavel/rumbac

An example usage session then looks like below:

_Remember to first double-press the button on the device
such that the LED will start pulsating._

```
C:> rumbac
Found 1 serial port:
 "COM13" = UsbPort(UsbPortInfo { vid: 9025, pid: 90, serial_number: Some("00000000000000007F65FDABFE86A5EB"), manufacturer: Some("Microsoft"), product: Some("Urządzenie szeregowe USB (COM13)") })

C:> rumbac COM13
Initializing "COM13"...
> V#
< Arduino Bootloader (SAM-BA extended) 2.0 [Arduino:IKXYZ]
> I#
< nRF52840-QIAA
Feats { chip_erase: true, write_buffer: true, checksum_buffer: true, identify_chip: true, reset: true }
Flash { name: "nRF52840-QIAA", addr: 0, pages: 256, size: 4096, planes: 1, lock_regions: 0, user: 0, stack: 0 }

C:> rumbac COM13 myprogram.bin
Initializing "COM13"...
> V#
< Arduino Bootloader (SAM-BA extended) 2.0 [Arduino:IKXYZ]
> I#
< nRF52840-QIAA
> N#
< "\n\r"
> S00000000,00001000#
> Y00000000,0#
< "Y\n\r"
> Y00000000,00001000#
< "Y\n\r"
> K#
```

_The device should now automatically disconnect and start running
the newly flashed program._

