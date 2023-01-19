use crate::server::HardwareRequest;
use eyre::Result;
use postcard::{from_bytes, to_slice};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use eyre::eyre;
use tokio::sync::oneshot;
use tokio_serial::{SerialPortType, SerialStream};
use tracing::{info, trace, debug, span, error, warn, Level};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
enum Operation {
    KeepAlive,
    SabertoothWrite(u8, u8),
    SmartelexWrite(u8, [u8; 5]),
    EncoderRead,
    PwmWrite(u8, u16),
    VersionReport,
    Reset,
}

#[derive(Debug)]
pub struct PadRequest {
    pub id: u8,
    pub body: HardwareRequest,
    tx: tokio::sync::oneshot::Sender<PadResponse>,
}
impl PadRequest {
    pub fn from_hardware_request(
        id: u8,
        hwrq: HardwareRequest,
    ) -> (tokio::sync::oneshot::Receiver<PadResponse>, Self) {
        let (pad_tx, server_rx) = tokio::sync::oneshot::channel();
        (
            server_rx,
            Self {
                id,
                body: hwrq,
                tx: pad_tx,
            },
        )
    }
}
#[derive(Debug, Clone)]
pub enum PadResponse {
    EncoderValue(i32),
    Ok,
}

pub struct PadState {
    serial: Option<SerialStream>,
    pwm_freq: u32,
    pwm_adc_max_value: u16,
}
impl PadState {
    pub fn new() -> Self {
        Self { serial: None, pwm_freq: 60, pwm_adc_max_value: 4095 }
    }
    pub async fn connect_device(&mut self) {
        const VID: u16 = 0x2E8A;
        const PID: u16 = 0x000A;
        if let Err(e) = serialport::available_ports() {
            error!("Error listing serial ports: {}", e);
            return;
        }
        let ports = serialport::available_ports().unwrap();
        if ports.is_empty() {
            error!("No serial ports found!");
            return;
        }
        for port in ports {
            debug!("Found port: {:?}", port);
            if let SerialPortType::UsbPort(info) = &port.port_type {
                if info.vid == VID && info.pid == PID {
                    info!("Found pad device!");
                    self.setup_serial(&port).await.map_err(|e| error!("Error setting up serial port: {}", e)).ok();
                }
            }
        }
    }
    async fn setup_serial(&mut self, port: &serialport::SerialPortInfo) -> Result<()> {
        self.serial = Some(
            SerialStream::open(
                &serialport::new(&port.port_name, 9600)
                    .timeout(std::time::Duration::from_millis(1000)),
            )?
        );
        debug!("Trying to get version");
        let mut buf = [0u8; 64];
        let op = Operation::VersionReport;
        let coded = to_slice(&op, &mut buf)?;
        self.serial.as_mut().ok_or_else(|| eyre!("No PAD serial device found"))?.write_all(coded).await?;
        let read = self.serial.as_mut().ok_or_else(|| eyre!("No PAD serial device found"))?.read(&mut buf).await?;
        let pad_version = String::from_utf8(buf[..read].to_vec())?;
        info!("PAD reported version: {}", pad_version);
        Ok(())
    }
    pub async fn keep_alive(&mut self) -> Result<()> {
        let _span_ = span!(Level::TRACE, "PadState::keep_alive").entered();
        let mut buf = [0u8; 64];
        let op = Operation::KeepAlive;
        let coded = to_slice(&op, &mut buf)?;
        self.serial.as_mut().ok_or_else(|| eyre!("No PAD serial device found"))?.write_all(coded).await?;
        trace!("Sent keep alive");
        trace!("Written bytes: {:?}", coded);
        Ok(())
    }
    fn microseconds_to_analog_value(&self, microseconds: u16) -> u16 {
        let microseconds = microseconds as f32;
        let microseconds = microseconds / 1000_000.0;
        let microseconds = microseconds * self.pwm_freq as f32;
        let microseconds = microseconds * self.pwm_adc_max_value as f32;
        microseconds as u16
    }
    pub async fn respond(
        &mut self,
        pad_rq: PadRequest,
    ) -> Result<(oneshot::Sender<PadResponse>, PadResponse)> {
        let _span_ = span!(Level::TRACE, "PadState::respond", pad_rq = ?pad_rq).entered();
        match pad_rq.body {
            HardwareRequest::ServoWrite { servo: _, position: value } => {
                let op = Operation::PwmWrite(pad_rq.id, self.microseconds_to_analog_value(value));
                let mut buf = [0u8; 64];
                let coded = to_slice(&op, &mut buf)?;
                self.serial.as_mut().ok_or_else(|| eyre!("No PAD serial device found"))?.write_all(coded).await?;
                debug!("Written servo: {:?}", coded);
                Ok((pad_rq.tx, PadResponse::Ok))
            }
            HardwareRequest::MotorWrite { motor: _, command } => {
                let op = match command.len() {
                    1 => Operation::SabertoothWrite(pad_rq.id, command[0]),
                    5 => Operation::SmartelexWrite(pad_rq.id, command.as_slice().try_into()?),
                    _ => panic!("MotorWrite Command received has invalid command length. Expected 1 or 5, got {}. Command: {:?}", command.len(), command),
                };
                let mut buf = [0u8; 64];
                let coded = to_slice(&op, &mut buf)?;
                self.serial.as_mut().ok_or_else(|| eyre!("No PAD serial device found"))?.write_all(coded).await?;
                debug!("Written bytes: {:?}", coded);
                Ok((pad_rq.tx, PadResponse::Ok))
            }
            HardwareRequest::EncoderRead { encoder: _ } => {
                let op = Operation::EncoderRead;
                let mut buf = [0u8; 64];
                let coded = to_slice(&op, &mut buf)?;
                self.serial.as_mut().ok_or_else(|| eyre!("No PAD serial device found"))?.write_all(coded).await?;
                let read = self.serial.as_mut().ok_or_else(|| eyre!("No PAD serial device found"))?.read(&mut buf).await?;
                let encoder_values: [i32; 5] = from_bytes(&buf[..read])?;
                debug!("Encoder values: {:?}", encoder_values);
                Ok((
                    pad_rq.tx,
                    PadResponse::EncoderValue(encoder_values[pad_rq.id as usize]),
                ))
            }
            HardwareRequest::PadReset => {
                let op = Operation::Reset;
                let mut buf = [0u8; 64];
                let coded = to_slice(&op, &mut buf)?;
                self.serial.as_mut().ok_or_else(|| eyre!("No PAD serial device found"))?.write_all(coded).await?;
                debug!("Written bytes: {:?}", coded);
                Ok((pad_rq.tx, PadResponse::Ok))
            }
            _ => {warn!("PadState::respond: Unimplemented request: {:?}", pad_rq.body); Ok((pad_rq.tx, PadResponse::Ok))}
        }
    }
}
