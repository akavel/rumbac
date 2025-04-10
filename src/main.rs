fn main() {
    let ports = serialport::available_ports().expect("failed to read serial ports");
    println!("Found {} serial ports.", ports.len());
    for p in ports {
        println!("port: {p:?}");
    }
}
