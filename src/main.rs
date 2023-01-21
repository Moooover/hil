// Setup a tokio server which listens to UNIX socket connections
mod config;
mod local;
mod pad;
mod server;
use eyre::{Result, WrapErr};
use std::sync::Arc;
use tokio::net::UnixListener;
use tracing::{debug, error, info, warn};

use git_version::git_version;
const GIT_VERSION: &str = git_version!();

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
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
    let (send_to_local, mut recv_from_server_local) =
        tokio::sync::mpsc::channel::<local::LocalRequest>(100);

    let mut local_connections = local::LocalConnections::from_config(&config).await;
    local_connections.setup_pins()?;
    let local_connections_handle = tokio::spawn(async move {
        loop {
            let request = recv_from_server_local.recv().await.unwrap();
            let (send_to_server, response) = local_connections
                .respond(request)
                .wrap_err("Error responging to a LocalRequest")
                .unwrap();
            if matches!(response, local::LocalResponse::SwitchOn(_))
                && send_to_server.send(response).is_err()
            {
                error!("Could not send back Switch state, receiver dropped.");
                break;
            }
        }
    });
    let server_handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
                Ok(accept_result) => {
                    let pad_channel = send_to_pad.clone();
                    let local_channel = send_to_local.clone();
                    let config = config.clone();
                    tokio::spawn(async move {
                        server::handle_stream(&config, accept_result, pad_channel, local_channel)
                            .await
                            .map_err(|e| error!("Error handling stream: {}", e))
                            .ok();
                    });
                }
            };
        }
    });

    let mut interval = tokio::time::interval(std::time::Duration::from_millis(800));
    let mut pad = pad::PadState::new();
    pad.connect_device().await;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if pad.keep_alive().await.map_err(|e| error!("Error sending KeepAlive: {}", e)).is_err() {
                    warn!("Lost connection to PAD, trying to reconnect...");
                    pad.connect_device().await;
                }
            }
            pad_req = recv_from_server.recv() => {
                debug!("Got request from server: {:?}", pad_req);
                let (send_to_server, response) = pad.respond(pad_req.unwrap()).await.wrap_err("Error responding to pad request").unwrap();
                if matches!(response, pad::PadResponse::EncoderValue(_)) &&
                    send_to_server.send(response).is_err() {
                        error!("Could not send back encoder values, receiver dropped.");
                        break;
                }
            }
        }
    }
    local_connections_handle.await.unwrap();
    server_handle.await.unwrap();
    Ok(())
}
