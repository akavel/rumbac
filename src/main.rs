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

use serialport::SerialPort;
use std::io::Write;
use std::str::FromStr;

fn main() {
    let flags = flags::Rumbac::from_env_or_exit();

    use flags::RumbacCmd::*;
    match flags.subcommand {
        List(_) => {
            let ports = serialport::available_ports().expect("Failed to read serial ports");
            println!("Found {} serial ports.", ports.len());
            for p in ports {
                println!("port: {p:?}");
            }
        }
        Info(flags::Info { port }) => {
            // TODO: what baudrate to use by default??
            // let bauds = 921600u32;
            let bauds = 230400u32;
            use core::time::Duration;
            let mut port: Port = serialport::new(port, bauds)
                .timeout(Duration::from_secs(1))
                .open()
                .expect("Failed to open port")
                .into();

            // TODO: set binary mode

            // "version" info
            port.write("V#");
            let version = port.read_str();
            // parse "version" info
            const FEATS_PREFIX: &str = "[Arduino:";
            const FEATS_SUFFIX: &str = "]";
            let feats_idx =
                version.find(FEATS_PREFIX).expect("No feats prefix found") + FEATS_PREFIX.len();
            let feats = &version[feats_idx..];
            let feats_end = feats.find(FEATS_SUFFIX).expect("No feats end found");
            let feats: Feats = feats[..feats_end].parse().unwrap();
            println!("{feats:?}");

            let mut flash = None;
            if feats.identify_chip {
                port.write("I#");
                let ident = port.read_str();
                if ident == FAMILY_NRF52 {
                    flash = Some(Flash {
                        name: FAMILY_NRF52.into(),
                        addr: 0,
                        pages: 256,
                        size: 4096,
                        planes: 1,
                        lock_regions: 0,
                        user: 0,
                        stack: 0,
                    });
                }
            }
            println!("{flash:?}");
        }
    }
}

struct Port {
    inner: Box<dyn serialport::SerialPort>,
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
        println!("> {}", s);
        let _ = self
            .inner
            .write(s.as_bytes())
            .expect("Failed to write to port");
    }

    pub fn read_str(&mut self) -> String {
        let mut buf = Vec::new();
        buf.resize(256, b' ');

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

        buf.pop_if(|b| *b == b'\0');
        buf.pop_if(|b| *b == b'\r');
        buf.pop_if(|b| *b == b'\n');
        let line = std::str::from_utf8(&buf).expect("Cannot parse as UTF8");
        println!("< {line}");
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
    size: u32,
    planes: u32,
    lock_regions: u32,
    user: u32,
    stack: u32,
}

mod flags {
    xflags::xflags! {
        src "./src/main.rs"

        cmd rumbac {
            default cmd list { }
            cmd info {
                required -p,--port port: String
            }
        }
    }
    // generated start
    // The following code is generated by `xflags` macro.
    // Run `env UPDATE_XFLAGS=1 cargo build` to regenerate.
    #[derive(Debug)]
    pub struct Rumbac {
        pub subcommand: RumbacCmd,
    }

    #[derive(Debug)]
    pub enum RumbacCmd {
        List(List),
        Info(Info),
    }

    #[derive(Debug)]
    pub struct List;

    #[derive(Debug)]
    pub struct Info {
        pub port: String,
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
