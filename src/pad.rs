use crate::server::HardwareRequest;
use postcard::{from_bytes, to_slice};
use serde::{Deserialize, Serialize};
use serialport::{SerialPort, SerialPortType};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
enum Operation {
    KeepAlive,
    SabertoothWrite(u8, u8),
    SmartelexWrite(u8, [u8; 5]),
    EncoderRead,
    PwmWrite(u8, u16),
}

#[derive(Debug)]
pub struct PadRequest {
    pub id: u8,
    pub body: HardwareRequest,
}
impl PadRequest {
    pub fn from_hardware_request(id: u8, hwrq: HardwareRequest) -> Self {
        Self { id, body: hwrq }
    }
}
#[derive(Debug)]
pub enum PadResponse {
    EncoderValue(i32),
    Ok,
}

pub struct PadState {
    serial: Option<Box<dyn SerialPort>>,
}
impl PadState {
    pub fn new() -> Self {
        Self { serial: None }
    }
    pub fn connect_device(&mut self) {
        const VID: u16 = 0x2E8A;
        const PID: u16 = 0x000A;
        if let Err(e) = serialport::available_ports() {
            println!("Error listing serial ports: {}", e);
            return;
        }
        let ports = serialport::available_ports().unwrap();
        if ports.len() == 0 {
            println!("No serial ports found!");
            return;
        }
        for port in ports {
            println!("Found port: {:?}", port);
            if let SerialPortType::UsbPort(info) = &port.port_type {
                if info.vid == VID && info.pid == PID {
                    println!("Found pad device!");
                    self.setup_serial(&port);
                }
            }
        }
    }
    fn setup_serial(&mut self, port: &serialport::SerialPortInfo) {
        self.serial = Some(serialport::new(&port.port_name, 9600).open().unwrap());
        self.serial
            .as_mut()
            .unwrap()
            .set_timeout(std::time::Duration::from_millis(1000))
            .unwrap();
    }
    pub fn keep_alive(&mut self) {
        let mut buf = [0u8; 64];
        let op = Operation::KeepAlive;
        let coded = to_slice(&op, &mut buf).unwrap();
        self.serial.as_mut().unwrap().write(&coded).unwrap();
        println!("Written bytes: {:?}", coded);
    }
    pub fn respond(&mut self, pad_rq: PadRequest) -> PadResponse {
        match pad_rq.body {
            HardwareRequest::MotorWrite { motor: _, command } => {
                let op = Operation::SabertoothWrite(pad_rq.id, command[0]);
                let mut buf = [0u8; 64];
                let coded = to_slice(&op, &mut buf).unwrap();
                self.serial.as_mut().unwrap().write(&coded).unwrap();
                println!("Written bytes: {:?}", coded);
                PadResponse::Ok
            }
            HardwareRequest::EncoderRead { encoder: _ } => {
                let op = Operation::EncoderRead;
                let mut buf = [0u8; 64];
                let coded = to_slice(&op, &mut buf).unwrap();
                self.serial.as_mut().unwrap().write(&coded).unwrap();
                println!("Written bytes: {:?}", coded);
                let read = self.serial.as_mut().unwrap().read(&mut buf).unwrap();
                let encoder_values: [i32; 5] = from_bytes(&buf[..read]).unwrap();
                PadResponse::EncoderValue(encoder_values[pad_rq.id as usize])
            }
        }
    }
}
