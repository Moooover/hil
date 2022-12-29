use crate::config::{Config, Handler};
use crate::pad::{PadRequest, PadResponse};
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::io;
use tokio::net::{unix::SocketAddr, UnixStream};
use tracing::{debug, error, info, warn};

#[derive(Serialize, Deserialize, Debug)]
pub enum HardwareRequest {
    MotorWrite { motor: String, command: Vec<u8> },
    EncoderRead { encoder: String },
}
#[derive(Serialize, Deserialize, Debug)]
pub enum HardwareResponse {
    EncoderValue(i32),
    Ok,
}
impl HardwareResponse {
    pub fn from_pad_response(pr: PadResponse) -> Self {
        match pr {
            PadResponse::EncoderValue(v) => Self::EncoderValue(v),
            PadResponse::Ok => Self::Ok,
        }
    }
}

#[derive(Debug)]
pub struct ServerChannels {
    pub send_to_pad: tokio::sync::mpsc::Sender<PadRequest>,
    pub recv_from_pad: tokio::sync::mpsc::Receiver<PadResponse>,
}
#[tracing::instrument]
pub async fn handle_stream(
    config: &Config,
    accept_result: Result<(UnixStream, SocketAddr), io::Error>,
    channels: &mut ServerChannels,
) -> Result<()> {
    if let Err(e) = accept_result {
        error!("Error accepting connection: {}", e);
        return Ok(());
    }
    let (stream, _addr) = accept_result?;
    info!("New connection: {:?}", stream);
    while stream.readable().await.is_ok() {
        let mut msg = vec![0; 1024];
        match stream.try_read(&mut msg) {
            Ok(n) => {
                debug!("Read {} bytes", n);
                // let sample_msg = HardwareRequest::MotorWrite{motor: "motor1".to_string(), command: vec![0x2A, 0x08, 0xFF, 0xFF, 0x23]};
                // let encoded_msg = serde_json::to_string(&sample_msg).unwrap();
                // debug!("Encoded message: {:?}", encoded_msg);
                let hw_req: HardwareRequest = serde_json::from_slice(&msg[..n])?;
                debug!("Message: {:?}", hw_req);

                if let HardwareResponse::EncoderValue(v) =
                    handle_request(config, hw_req, channels).await
                {
                    let encoded_resp = serde_json::to_string(&v)?;
                    debug!("Encoded response: {:?}", encoded_resp);
                    let resp = encoded_resp.as_bytes();
                    stream.writable().await?;

                    match stream.try_write(resp) {
                        Ok(n) => {
                            debug!("Wrote {} bytes", n);
                        }
                        Err(e) => {
                            error!("Error writing to stream: {}", e);
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                info!("Would block");
            }
            Err(e) => {
                error!("Error: {:?}", e);
                return Err(e.into());
            }
        }
    }
    Ok(())
}
#[tracing::instrument]
async fn handle_request(
    config: &Config,
    req: HardwareRequest,
    channels: &mut ServerChannels,
) -> HardwareResponse {
    match config.resolve(&req) {
        Some(Handler::Pad(port)) => {
            let wait_for_response = matches!(req, HardwareRequest::EncoderRead { .. });
            let pad_req = PadRequest::from_hardware_request(port, req);
            channels.send_to_pad.send(pad_req).await.unwrap();
            if wait_for_response {
                let pad_resp = channels.recv_from_pad.recv().await.unwrap();
                debug!("Received pad response: {:?}", pad_resp);
                HardwareResponse::from_pad_response(pad_resp)
            } else {
                HardwareResponse::Ok
            }
        }
        Some(Handler::System(port)) => {
            warn!("System port: {}, not implemented", port);
            HardwareResponse::Ok
        }
        None => {
            warn!("No handler found");
            HardwareResponse::Ok
        }
    }
}
