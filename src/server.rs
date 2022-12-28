use tokio::net::{UnixStream, unix::SocketAddr};
use serde::{Serialize, Deserialize};
use std::io;
use crate::config::{Config, Handler};
use crate::pad;
use tracing::{info, error, debug};

#[derive(Serialize,Deserialize,Debug)]
pub enum HardwareRequest {
    MotorWrite{motor: String, command: Vec<u8>},
    EncoderRead{encoder: String},
}
#[tracing::instrument]
pub async fn handle_stream(config: &Config, accept_result: Result<(UnixStream, SocketAddr), io::Error>, channels: &mut ServerChannels) {
    if let Err(e) = accept_result {
        error!("Error accepting connection: {}", e);
        return;
    }
    let (stream, _addr) = accept_result.unwrap();
            info!("New connection: {:?}", stream);
            stream.readable().await.unwrap();
            let mut msg = vec![0; 1024];
            match stream.try_read(&mut msg) {
                Ok(n) => {
                    debug!("Read {} bytes", n);
                    let sample_msg = HardwareRequest::MotorWrite{motor: "motor1".to_string(), command: vec![0x2A, 0x08, 0xFF, 0xFF, 0x23]};
                    let encoded_msg = serde_json::to_string(&sample_msg).unwrap();
                    debug!("Encoded message: {:?}", encoded_msg);
                    let hw_req: HardwareRequest = serde_json::from_slice(&msg[..n]).unwrap();
                    debug!("Message: {:?}", hw_req);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    info!("Would block");
                }
                Err(e) => {
                    error!("Error: {:?}", e);
                }
            }
}
async fn handle_request(config: &Config, req: HardwareRequest) {
    match config.resolve(&req) {
        Some(Handler::Pad(port)) => {
            println!("Pad port: {}", port);
        }
        Some(Handler::System(port)) => {
            println!("System port: {}", port);
        }
        None => {
            println!("No handler found");
        }
    }
}
