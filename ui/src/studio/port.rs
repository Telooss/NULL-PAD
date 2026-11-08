//! Port série du pad : détection (VID/PID Zephyr 1D50:615E) et ouverture.
use serialport::SerialPortType;
use std::io::{Read, Write};
use std::time::Duration;

pub trait Port: Read + Write + Send {}
impl<T: Read + Write + Send> Port for T {}

const VID: u16 = 0x1D50;
const PID: u16 = 0x615E;

pub fn find_pad_port() -> Option<String> {
    serialport::available_ports()
        .ok()?
        .into_iter()
        .find(
            |p| matches!(&p.port_type, SerialPortType::UsbPort(u) if u.vid == VID && u.pid == PID),
        )
        .map(|p| p.port_name)
}

pub fn open_pad(name: &str) -> serialport::Result<Box<dyn Port>> {
    let port = serialport::new(name, 115_200)
        .timeout(Duration::from_millis(100))
        .open()?;
    Ok(Box::new(port))
}
