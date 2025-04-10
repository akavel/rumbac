fn main() {
    let flags = flags::Rumbac::from_env_or_exit();
    println!("{flags:?}");

    let ports = serialport::available_ports().expect("failed to read serial ports");
    println!("Found {} serial ports.", ports.len());
    for p in ports {
        println!("port: {p:?}");
    }
}

mod flags {
    xflags::xflags! {
        cmd rumbac {
            default cmd list { }
            cmd info {
                required -p,--port port: String
            }
        }
    }
}
