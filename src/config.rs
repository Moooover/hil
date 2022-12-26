use figment::{Figment, providers::{Format, Toml}};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct PadConfig {
    motors: HashMap<String, u8>,
    encoders: HashMap<String, u8>,
}
#[derive(Deserialize, Debug)]
pub struct SystemConfig {
    motors: HashMap<String, u8>,
    encoders: Option<HashMap<String, u8>>,
}
#[derive(Deserialize, Debug)]
pub struct Config {
    pad: PadConfig,
    system: SystemConfig,
}
pub fn load_config() {
    let config = Figment::new()
        .merge(Toml::file("config.toml").nested());
    let pad_config: SystemConfig = config.select("system").extract().unwrap();
    println!("{:#?}", pad_config);
}
