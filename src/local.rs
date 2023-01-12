use sysfs_gpio::{Direction, Pin};
use eyre::{Result, Error};
use tokio::time::{sleep, Duration};
use crate::config::SystemConfig;
use crate::server::HardwareRequest;

#[derive(Debug)]
struct LocalConnections {
    limit_switches: [Pin; 5],
    h_bridge: [Pin; 4],
    status_leds: [Pin; 3]
}

#[derive(Debug)]
struct LocalRequest {
    pub id: u8,
    pub body: HardwareRequest
}

#[derive(Debug)]
enum LocalResponse {
    SwitchOn(bool),
    Ok,
}

impl LocalConnections {
    pub async fn from_config(config: &SystemConfig) -> Self {
        let limit_switches: [Pin; 5] = config.limit_switches.iter().map(|(_, pin)| Pin::new(*pin)).collect::<Vec<_>>().try_into().unwrap();
        let h_bridge = config.motors.iter().map(|(_, pin)| Pin::new(*pin)).collect::<Vec<_>>().try_into().unwrap();
        let status_leds = config.status_leds.iter().map(|(_, pin)| Pin::new(*pin)).collect::<Vec<_>>().try_into().unwrap();
        // udev takes ~80ms to export the pins
        sleep(Duration::from_millis(100)).await;
        Self {
            limit_switches,
            h_bridge,
            status_leds
        }
    }

    pub fn setup_pins(&mut self) -> Result<()> {
        for input_pin in self.limit_switches.iter_mut() {
            input_pin.set_direction(Direction::In)?;
        }
        for output_pin in self.h_bridge.iter_mut() {
            output_pin.set_direction(Direction::Out)?;
        }
        for output_pin in self.status_leds.iter_mut() {
            output_pin.set_direction(Direction::Out)?;
        }
        Ok(())
    }

    pub fn respond(&mut self, lrq: LocalRequest) -> Result<LocalResponse> {
        match lrq.body {
            HardwareRequest::SwitchRead { switch: _ } => {
                let pin = self.limit_switches.get(lrq.id as usize).ok_or(Error::msg("Invalid switch id"))?;
                let value = pin.get_value()?;
                Ok(LocalResponse::SwitchOn(value == 1))
            },
            HardwareRequest::LedWrite { led: _, state } => {
                let pin = self.status_leds.get(lrq.id as usize).ok_or(Error::msg("Invalid led id"))?;
                pin.set_value(state)?;
                Ok(LocalResponse::Ok)
            },
            HardwareRequest::MotorWrite { motor: _, command } => {
                let pins = self.h_bridge.get(lrq.id as usize).ok_or(Error::msg("Invalid h-bridge id"))?;
                let value = command[0];
                pins.set_value(value)?;
                Ok(LocalResponse::Ok)
            },
            _ => Err(Error::msg("Could not handle request locally"))
        }
    }

    fn write_h_bridge(&mut self, command: &Vec<u8>) -> Result<()> {
        for (pin, value) in self.h_bridge.iter_mut().zip(command.iter()) {
            pin.set_value(*value)?;
        }
        Ok(())
    }
}
