use crate::server::HardwareRequest;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use tracing::info;

#[derive(Default, Deserialize, Debug)]
pub struct PadConfig {
    motors: HashMap<String, u8>,
    encoders: HashMap<String, u8>,
    servos: HashMap<String, u8>,
}
#[derive(Deserialize, Debug, Clone)]
pub struct SystemConfig {
    pub motors: HashMap<String, [u64; 2]>,
    pub limit_switches: HashMap<String, u64>,
    pub status_leds: HashMap<String, u64>,
}
#[derive(Deserialize, Debug)]
pub struct Config {
    pub pad: PadConfig,
    pub system: SystemConfig,
}
pub enum Handler {
    Pad(u8),
    System,
}

impl Config {
    pub fn resolve(&self, hrq: &HardwareRequest) -> Option<Handler> {
        match hrq {
            HardwareRequest::ServoWrite { servo, position: _ } => self.pad.servos.get(servo).map(|port| Handler::Pad(*port)),
            HardwareRequest::MotorWrite { motor, command: _ } => self
                .pad
                .motors
                .get(motor)
                .map(|port| Handler::Pad(*port))
                .or_else(|| {
                    self.system
                        .motors
                        .get(motor)
                        .map(|_| Handler::System)
                }),
            HardwareRequest::EncoderRead { encoder } => self
                .pad
                .encoders
                .get(encoder)
                .map(|port| Handler::Pad(*port)),
            HardwareRequest::PadReset => Some(Handler::Pad(0)),
            HardwareRequest::SwitchRead {switch: _} | HardwareRequest::LedWrite {led: _, state: _} => Some(Handler::System),
        }
    }
}
pub fn load_config() -> Config {
    let config_file_path = xdg::BaseDirectories::with_prefix("spine")
        .unwrap()
        .find_config_file("config.toml")
        .unwrap();
    let config_file = File::open(config_file_path).unwrap();
    let mut buf_reader = BufReader::new(config_file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents).unwrap();

    let config: Config = toml::from_str(&contents).unwrap();
    info!("{:#?}", config);
    config
}
