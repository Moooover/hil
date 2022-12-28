use serde::Deserialize;
use std::collections::HashMap;
use crate::server::HardwareRequest;
use tracing::debug;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

#[derive(Default, Deserialize, Debug)]
pub struct PadConfig {
    motors: HashMap<String, u8>,
    encoders: HashMap<String, u8>,
}
#[derive(Deserialize, Debug)]
pub struct SystemConfig {
    motors: HashMap<String, u8>,
}
#[derive(Deserialize, Debug)]
pub struct Config {
    pad: PadConfig,
    system: SystemConfig,
}
pub enum Handler {
    Pad(u8),
    System(u8),
}

impl Config {
    pub fn resolve(&self, hrq: &HardwareRequest) -> Option<Handler> {
        match hrq {
            HardwareRequest::MotorWrite{motor, command} => {
                self.pad.motors.get(motor).map(|port| Handler::Pad(*port)).or_else(|| self.system.motors.get(motor).map(|port| Handler::System(*port)))
            }
            HardwareRequest::EncoderRead{encoder} => {
                self.pad.encoders.get(encoder).map(|port| Handler::Pad(*port))
            }
        }
    }
}
#[tracing::instrument]
pub fn load_config() -> Config {
    let config_file_path = xdg::BaseDirectories::with_prefix("spine").unwrap().find_config_file("config.toml").unwrap();
    let config_file = File::open(config_file_path).unwrap();
    let mut buf_reader = BufReader::new(config_file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents).unwrap();

    let config: Config = toml::from_str(&contents).unwrap();
    debug!("{:#?}", config);
    config
}
