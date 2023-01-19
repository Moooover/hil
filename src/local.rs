use sysfs_gpio::{Direction, Pin};
use eyre::{Result, Error};
use tracing::{debug};
use tokio::time::{sleep, Duration};
use tokio::sync::oneshot;
use std::collections::HashMap;
use crate::config::Config;
use crate::server::HardwareRequest;

type HBridgePinPair = [Pin; 2];
#[derive(Debug)]
pub struct LocalConnections {
    limit_switches: HashMap<String, Pin>,
    h_bridge: HashMap<String, HBridgePinPair>,
    status_leds: HashMap<String, Pin>,
}

#[derive(Debug)]
pub struct LocalRequest {
    pub body: HardwareRequest,
    tx: tokio::sync::oneshot::Sender<LocalResponse>,
}

#[derive(Debug)]
pub enum LocalResponse {
    SwitchOn(bool),
    Ok,
}

impl LocalConnections {
    pub async fn from_config(config: &Config) -> Self {
        let mut config = config.system.clone();
        let limit_switches: HashMap<String, Pin> = config
            .limit_switches
            .drain()
            .map(|(name, pin)| (name, Pin::new(pin as u64)))
            .collect();
        let h_bridge: HashMap<String, HBridgePinPair> = config
            .motors
            .drain()
            .map(|(name, pins)| {
                (name, [Pin::new(pins[0]), Pin::new(pins[1])])
            })
            .collect();
        let status_leds: HashMap<String, Pin> = config.status_leds.drain().map(|(name, pin)| (name, Pin::new(pin))).collect();
        // udev takes ~80ms to export the pins
        
        limit_switches.values().for_each(|pin| pin.export().unwrap());
        h_bridge.values().for_each(|pins| pins.iter().for_each(|pin| pin.export().unwrap()));
        status_leds.values().for_each(|pin| pin.export().unwrap());
        sleep(Duration::from_millis(100)).await;
        Self {
            limit_switches,
            h_bridge,
            status_leds
        }
    }

    pub fn setup_pins(&mut self) -> Result<()> {
        for input_pin in self.limit_switches.values_mut() {
            debug!("Setting up pin {:?}", input_pin);
            input_pin.set_direction(Direction::In)?;
        }
        for output_pin in self.h_bridge.values_mut() {
            debug!("Setting up motor pins {:?}", output_pin);
            output_pin[0].set_direction(Direction::Out)?;
            output_pin[1].set_direction(Direction::Out)?;
        }
        for output_pin in self.status_leds.values_mut() {
            debug!("Setting up led pins {:?}", output_pin);
            output_pin.set_direction(Direction::Out)?;
        }
        Ok(())
    }

    pub fn respond(&mut self, lrq: LocalRequest) -> Result<(oneshot::Sender<LocalResponse>, LocalResponse)> {
        match lrq.body {
            HardwareRequest::SwitchRead { switch } => {
                let pin = self.limit_switches.get(&switch).ok_or(Error::msg("Invalid switch id"))?;
                let value = pin.get_value()?;
                Ok((lrq.tx, LocalResponse::SwitchOn(value == 1)))
            },
            HardwareRequest::LedWrite { led, state } => {
                let pin = self.status_leds.get(&led).ok_or(Error::msg("Invalid led id"))?;
                pin.set_value(state)?;
                Ok((lrq.tx, LocalResponse::Ok))
            },
            HardwareRequest::MotorWrite { motor, command } => {
                let h_bridge = self.h_bridge.get(&motor).ok_or(Error::msg("Invalid h-bridge id"))?;
                let value = command[0];
                self.write_h_bridge(*h_bridge, value)?;
                Ok((lrq.tx, LocalResponse::Ok))
            },
            _ => Err(Error::msg("Could not handle request locally"))
        }
    }

    fn write_h_bridge(&mut self, h_bridge: HBridgePinPair, command: u8) -> Result<()> {
        match command {
            65..=127 | 193..=u8::MAX => {
                h_bridge[0].set_value(1)?;
                h_bridge[1].set_value(0)?;
            },
            1..=63 | 128..=190 => {
                h_bridge[0].set_value(0)?;
                h_bridge[1].set_value(1)?;
            },
            // The reason why 191 is here, is due rounding down in affine_transform in wroom
            // With that, someone may think, that 191 corresponds to a rest position, which may be true
            // for Sabertooth, but for the H-Bridge, there's no speed control, only discrete
            // on/off.
            0 | 64 | 191 | 192 => {
                h_bridge[0].set_value(0)?;
                h_bridge[1].set_value(0)?;
            },
        }
        Ok(())
    }
}

impl LocalRequest {
    pub fn from_hardware_request(body: HardwareRequest) -> (tokio::sync::oneshot::Receiver<LocalResponse>, Self) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (rx, Self { body, tx })
    }
}
