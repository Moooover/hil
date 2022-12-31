// Setup a tokio server which listens to UNIX socket connections
mod config;
mod pad;
mod server;
use eyre::{WrapErr, Result};
use tokio::net::UnixListener;
use std::sync::Arc;
use tracing::{error, info, debug};

use git_version::git_version;
const GIT_VERSION: &str = git_version!();

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    info!("Starting spine version {}", GIT_VERSION);
    let config = Arc::new(config::load_config());
    // Check if file /tmp/hardware.sock exists, if so, delete it
    if std::path::Path::new("/tmp/hardware.sock").exists() {
        std::fs::remove_file("/tmp/hardware.sock")?
    }
    let listener = UnixListener::bind("/tmp/hardware.sock").unwrap();
    let (send_to_pad, mut recv_from_server) = tokio::sync::mpsc::channel::<pad::PadRequest>(100);
    let (send_to_server, recv_from_pad) = tokio::sync::mpsc::channel::<pad::PadResponse>(100);
    let mut server_channels = server::ServerChannels {
        send_to_pad,
    };
    // let (send_to_server, recv_from_pad) = tokio::sync::oneshot::channel();
    let server_handle = tokio::spawn(async move {
        loop {
            match listener.accept().await{
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
                Ok(accept_result) => {
                    let channels = server_channels.clone();
                    let config = config.clone();
            tokio::spawn(async move {
                server::handle_stream(&config, accept_result, channels).await.map_err(|e| error!("Error handling stream: {}", e)).ok();
            });

                }
            };
        }
    });

    let mut interval = tokio::time::interval(std::time::Duration::from_millis(800));
    let mut pad = pad::PadState::new();
    pad.connect_device();

    loop {
        tokio::select! {
            _ = interval.tick() => {
                pad.keep_alive().await.map_err(|e| error!("Error sending KeepAlive: {}", e)).ok();
            }
            pad_req = recv_from_server.recv() => {
                debug!("Got request from server: {:?}", pad_req);
                let (send_to_server, response) = pad.respond(pad_req.unwrap()).await.wrap_err("Error responding to pad request").unwrap();
                if matches!(response, pad::PadResponse::EncoderValue(_)) {
                    if let Err(_) = send_to_server.send(response) {
                        error!("Could not send back encoder values, receiver dropped.");
                        break;
                    }
                }
            }
        }
    }
    server_handle.await.unwrap();
    Ok(())
}
