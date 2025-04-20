// rumbac
//
// Copyright (c) 2025, Mateusz Czapliński "akavel"
// Copyright (c) 2018, ShumaTech
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
// along with this program.  If not, see <http://www.gnu.org/licenses/>

use anyhow::{Context, Result, bail};
use serialport::SerialPort;
use std::io::{Read, Write};
use std::str::FromStr;

fn main() {
    let flags = flags::Rumbac::from_env_or_exit();

    let Some(port) = flags.port else {
        // list known ports
        // TODO: make it prettier
        let ports = serialport::available_ports().expect("Failed to read serial ports");
        let n = ports.len();
        let ending = match n {
            0 => "s.",
            1 => ":",
            2.. => "s:",
        };
        println!("Found {n} serial port{ending}");
        if n == 0 {
            println!(
                "HINT: Did you press the magic combination of button(s) on your plugged-in device to put it in RESET / BOOT mode?"
            );
            return;
        }
        for p in ports {
            println!(" {:?} = {:?}", p.port_name, p.port_type);
        }
        return;
    };

    println!("Initializing {port:?}...");
    let (mut port, feats, flash) = init(&port).unwrap();

    let Some(file) = flags.file else {
        println!("{feats:?}");
        println!("{flash:?}");
        return;
    };
    let mut file = std::fs::File::open(file).expect("Cannot open input file");
    {
        let metadata = file.metadata().expect("Cannot retrieve file size");
        let size = metadata.len();
        let max_size = flash.pages as u64 * flash.size as u64;
        if size > max_size {
            panic!("File size {size} too big, must not exceed flash size {max_size}");
        }
    }

    // write file to flash
    if !feats.write_buffer {
        panic!("only write_buffer flashing method currently implemented");
    }
    port.write("N#");
    port.expect("\n\r");
    const WRITE_BUF_SIZE: u32 = 4096;
    let mut buf = vec![0u8; WRITE_BUF_SIZE as usize];
    let mut offset = 0u32;
    loop {
        let mut n = read_buf(&mut file, &mut buf).expect("Error reading input file") as u32;
        if n == 0 {
            break; // eof
        }
        if n < WRITE_BUF_SIZE {
            buf[n as usize..].fill(0u8);
        }
        let page_size = flash.size;
        if n < WRITE_BUF_SIZE {
            n = (n + page_size - 1) / page_size * page_size;
            if n > WRITE_BUF_SIZE {
                n = WRITE_BUF_SIZE;
            }
        }

        port.write(&format!("S{:08X},{n:08X}#", flash.user));
        let _ = port.inner.flush();
        port.write_all(&buf[..n as usize]);

        port.write(&format!("Y{:08X},0#", flash.user));
        port.expect("Y\n\r");

        let dst_addr = flash.addr + offset;
        port.write(&format!("Y{dst_addr:08X},{n:08X}#"));
        port.expect("Y\n\r");

        offset += n;
    }

    // TODO: verify (if flag set)

    if feats.reset {
        port.write("K#");
    }
}

fn init(port_name: &str) -> Result<(Port, Feats, Flash)> {
    // TODO: what baudrate to use by default??
    // let bauds = 921600u32;
    let bauds = 230400u32;
    use core::time::Duration;
    let mut port: Port = serialport::new(port_name, bauds)
        .timeout(Duration::from_secs(1))
        .open()
        .with_context(|| format!("Failed to open port {port_name}"))?
        .into();

    // get "version" info
    port.write("V#");
    let version = port.read_str();
    // parse "version" info
    const FEATS_PREFIX: &str = "[Arduino:";
    const FEATS_SUFFIX: &str = "]";
    let feats_idx = version
        .find(FEATS_PREFIX)
        .with_context(|| format!("No {FEATS_PREFIX:?} found in version info {version:?}"))?
        + FEATS_PREFIX.len();
    let feats = &version[feats_idx..];
    let feats_end = feats
        .find(FEATS_SUFFIX)
        .with_context(|| format!("No {FEATS_SUFFIX:?} found in version info {version:?}"))?;
    let feats: Feats = feats[..feats_end].parse().unwrap();

    if feats.identify_chip {
        port.write("I#");
        match port.read_str().as_ref() {
            FAMILY_NRF52 => {
                return Ok((
                    port,
                    feats,
                    Flash {
                        name: FAMILY_NRF52.into(),
                        addr: 0,
                        pages: 256,
                        size: 4096,
                        planes: 1,
                        lock_regions: 0,
                        user: 0,
                        stack: 0,
                    },
                ));
            }
            _ => (),
        }
    }
    bail!("Device at {port_name:?} not recognized");
}

struct Port {
    inner: Box<dyn SerialPort>,
}

impl From<Box<dyn SerialPort>> for Port {
    fn from(p: Box<dyn SerialPort>) -> Self {
        Self::new(p)
    }
}

impl Port {
    pub fn new(p: Box<dyn SerialPort>) -> Self {
        Self { inner: p }
    }

    pub fn write(&mut self, s: &str) {
        println!("> {:?}", s);
        self.write_all(s.as_bytes());
    }

    pub fn write_all(&mut self, buf: &[u8]) {
        let mut offset: usize = 0;
        while offset < buf.len() {
            offset += self
                .inner
                .write(&buf[offset..])
                .expect("Failed to write to port");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    pub fn expect(&mut self, response: &str) {
        let mut buf = vec![b' '; response.len()];
        let mut offset: usize = 0;
        while offset < buf.len() {
            offset += self
                .inner
                .read(&mut buf[offset..])
                .expect("Failed to read from port");
        }
        let line = std::str::from_utf8(&buf).expect("Cannot parse as UTF8");
        println!("< {line:?}");
        if line != response {
            panic!("got unexpected response");
        }
    }

    pub fn read_str(&mut self) -> String {
        let mut buf = vec![b' '; 256];

        let mut offset: usize = 0;
        loop {
            let n = self
                .inner
                .read(&mut buf[offset..])
                .expect("Failed to read from port");
            if let Some(idx) = buf[offset..offset + n].iter().position(|b| *b == 0) {
                buf.truncate(offset + idx);
                break;
            }
            offset += n;
            if offset == buf.len() {
                panic!("read_str buffer too small");
            }
        }

        let line = std::str::from_utf8(&buf).expect("Cannot parse as UTF8");
        println!("< {line:?}");
        buf.pop_if(|b| *b == b'\0');
        buf.pop_if(|b| *b == b'\r');
        buf.pop_if(|b| *b == b'\n');
        let line = std::str::from_utf8(&buf).expect("Cannot parse as UTF8");
        line.into()
    }
}

#[derive(Debug, Default)]
struct Feats {
    pub chip_erase: bool,
    pub write_buffer: bool,
    pub checksum_buffer: bool,
    pub identify_chip: bool,
    pub reset: bool,
}

#[derive(Debug)]
pub struct ParseFeatsError(pub u8);

impl FromStr for Feats {
    type Err = ParseFeatsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut feats = Self::default();
        for b in s.as_bytes() {
            match b {
                b'I' => feats.identify_chip = true,
                b'K' => feats.reset = true,
                b'X' => feats.chip_erase = true,
                b'Y' => feats.write_buffer = true,
                b'Z' => feats.checksum_buffer = true,
                _ => return Err(ParseFeatsError(*b)),
            }
        }
        Ok(feats)
    }
}

const FAMILY_NRF52: &str = "nRF52840-QIAA";

#[derive(Debug)]
struct Flash {
    name: String,
    addr: u32,
    pages: u32,
    size: u32, // page size
    planes: u32,
    lock_regions: u32,
    user: u32,
    stack: u32,
}

fn read_buf(r: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut off = 0usize;
    while off < buf.len() {
        use std::io::ErrorKind::*;
        match r.read(&mut buf[off..]) {
            Ok(n) => {
                off += n;
                if n == 0 {
                    return Ok(off);
                }
            }
            Err(e) => match e.kind() {
                UnexpectedEof => return Ok(off),
                Interrupted => continue,
                _ => return Err(e),
            },
        }
    }
    Ok(off)
}

mod flags {
    // Planned usage patterns:
    // $ rumbac        ## lists detected ports
    // $ rumbac $PORT  ## shows info about device on given port
    // $ rumbac $PORT $FILE.bin  ## flashes $FILE.bin to device
    xflags::xflags! {
        src "./src/main.rs"

        cmd rumbac {
            optional port: String
            optional file: String
            // Erase the flash - may speed up writing
            // optional -e,--erase
        }
    }
    // generated start
    // The following code is generated by `xflags` macro.
    // Run `env UPDATE_XFLAGS=1 cargo build` to regenerate.
    #[derive(Debug)]
    pub struct Rumbac {
        pub port: Option<String>,
        pub file: Option<String>,
    }

    impl Rumbac {
        #[allow(dead_code)]
        pub fn from_env_or_exit() -> Self {
            Self::from_env_or_exit_()
        }

        #[allow(dead_code)]
        pub fn from_env() -> xflags::Result<Self> {
            Self::from_env_()
        }

        #[allow(dead_code)]
        pub fn from_vec(args: Vec<std::ffi::OsString>) -> xflags::Result<Self> {
            Self::from_vec_(args)
        }
    }
    // generated end
}
