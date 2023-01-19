use crate::config::{Config, Handler};
use crate::pad::{PadRequest, PadResponse};
use crate::local::{LocalRequest, LocalResponse};
use eyre::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{unix::SocketAddr, UnixStream};
use tracing::{debug, error, info, warn};

#[derive(Serialize, Deserialize, Debug)]
pub enum HardwareRequest {
    MotorWrite { motor: String, command: Vec<u8> },
    ServoWrite { servo: String, position: u16 },
    EncoderRead { encoder: String },
    SwitchRead { switch: String },
    LedWrite { led: String, state: u8 },
}
#[derive(Serialize, Deserialize, Debug)]
pub enum HardwareResponse {
    EncoderValue(i32),
    SwitchOn(bool),
    Ok,
}
impl HardwareResponse {
    pub fn from_pad_response(pr: PadResponse) -> Self {
        match pr {
            PadResponse::EncoderValue(v) => Self::EncoderValue(v),
            PadResponse::Ok => Self::Ok,
        }
    }
    pub fn from_local_response(lr: LocalResponse) -> Self {
        match lr {
            LocalResponse::SwitchOn(v) => Self::SwitchOn(v),
            LocalResponse::Ok => Self::Ok,
        }
    }
}

pub async fn handle_stream(
    config: &Config,
    accept_result: (UnixStream, SocketAddr),
    mut send_to_pad: tokio::sync::mpsc::Sender<PadRequest>,
    mut send_to_local: tokio::sync::mpsc::Sender<LocalRequest>,
) -> Result<()> {
    let (mut stream, _addr) = accept_result;
    info!("New connection: {:?}", stream);
    let mut msg = vec![0; 1024];
    loop {
        let n = stream.read(&mut msg).await?;
        if n == 0 {
            info!("Connection closed");
            break;
        }
        debug!("Read {} bytes", n);
        let hw_req_stream =
            serde_json::Deserializer::from_slice(&msg).into_iter::<HardwareRequest>();
        for hw_req_unchecked in hw_req_stream {
            if hw_req_unchecked.is_err() {
                warn!("Error decoding message");
                continue;
            }
            let hw_req = hw_req_unchecked.unwrap();
            info!("Successfully received HardwareRequest message");
            debug!("Message: {:?}", hw_req);

            match handle_request(config, hw_req, &mut send_to_pad, &mut send_to_local).await {
                HardwareResponse::EncoderValue(v) => {
                    let encoded_resp = serde_json::to_string(&v)?;
                    info!("Received encoder value, writing back to client");
                    debug!("Encoded response: {:?}", encoded_resp);
                    let resp = encoded_resp.as_bytes();
                    stream.writable().await?;

                    if let Err(e) = stream.write_all(resp).await {
                        error!("Error writing to stream: {}", e);
                    }
                },
                HardwareResponse::SwitchOn(v) => {
                    let encoded_resp = serde_json::to_string(&v)?;
                    info!("Received switch value, writing back to client");
                    debug!("Encoded response: {:?}", encoded_resp);
                    let resp = encoded_resp.as_bytes();
                    stream.writable().await?;

                    if let Err(e) = stream.write_all(resp).await {
                        error!("Error writing to stream: {}", e);
                    }
                },
                _ => {}
            }
        }
    }
    Ok(())
}
async fn handle_request(
    config: &Config,
    req: HardwareRequest,
    send_to_pad: &mut tokio::sync::mpsc::Sender<PadRequest>,
    send_to_local: &mut tokio::sync::mpsc::Sender<LocalRequest>,
) -> HardwareResponse {
    match config.resolve(&req) {
        Some(Handler::Pad(port)) => {
            let wait_for_response = matches!(req, HardwareRequest::EncoderRead { .. });
            debug!("Sending request to pad");
            let (recv_from_pad, pad_req) = PadRequest::from_hardware_request(port, req);
            send_to_pad.send(pad_req).await.unwrap();
            if wait_for_response {
                let pad_resp = recv_from_pad.await.unwrap();
                info!("Heard back from PAD, writing back HardwareResponse");
                debug!("Received pad response: {:?}", pad_resp);
                HardwareResponse::from_pad_response(pad_resp)
            } else {
                HardwareResponse::Ok
            }
        }
        Some(Handler::System) => {
            let wait_for_response = matches!(req, HardwareRequest::SwitchRead { .. });
            debug!("Sending request to local system");
            let (recv_from_local, local_req) = LocalRequest::from_hardware_request(req);
            send_to_local.send(local_req).await.unwrap();
            if wait_for_response {
                let local_resp = recv_from_local.await.unwrap();
                info!("Heard back from local system, writing back HardwareResponse");
                debug!("Received local response: {:?}", local_resp);
                HardwareResponse::from_local_response(local_resp)
            } else {
                HardwareResponse::Ok
            }
        }
        None => {
            warn!("No handler found");
            HardwareResponse::Ok
        }
    }
}
