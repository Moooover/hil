use sysfs_gpio::{Direction, Pin};
use eyre::{Result, Error};
use crate::config::SystemConfig;

#[derive(Debug)]
struct LocalConnections {
    limit_switches: [Pin; 5],
    h_bridge: [Pin; 4],
    status_leds: [Pin; 3]
}

impl LocalConnections {
    fn from_config(config: &SystemConfig) -> Self {
        let limit_switches: [Pin; 5] = config.limit_switches.iter().map(|(_, pin)| Pin::new(*pin)).collect::<Vec<_>>().try_into().unwrap();
        let h_bridge = config.motors.iter().map(|(_, pin)| Pin::new(*pin)).collect::<Vec<_>>().try_into().unwrap();
        let status_leds = config.status_leds.iter().map(|(_, pin)| Pin::new(*pin)).collect::<Vec<_>>().try_into().unwrap();
        Self {
            limit_switches,
            h_bridge,
            status_leds
        }
    }
}
