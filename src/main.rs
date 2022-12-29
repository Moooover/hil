// Setup a tokio server which listens to UNIX socket connections
mod config;
mod pad;
mod server;
use eyre::{WrapErr, Result};
use postcard::{from_bytes, to_slice};
use serde::{Deserialize, Serialize};
use tokio::net::UnixListener;
use tracing::{error, info};

use git_version::git_version;
const GIT_VERSION: &str = git_version!();

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    info!("Starting spine version {}", GIT_VERSION);
    let config = config::load_config();
    // Check if file /tmp/hardware.sock exists, if so, delete it
    if std::path::Path::new("/tmp/hardware.sock").exists() {
        std::fs::remove_file("/tmp/hardware.sock")?
    }
    let listener = UnixListener::bind("/tmp/hardware.sock").unwrap();
    let (send_to_pad, mut recv_from_server) = tokio::sync::mpsc::channel::<pad::PadRequest>(100);
    let (send_to_server, recv_from_pad) = tokio::sync::mpsc::channel::<pad::PadResponse>(100);
    let mut server_channels = server::ServerChannels {
        send_to_pad,
        recv_from_pad,
    };
    // let (send_to_server, recv_from_pad) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        loop {
            server::handle_stream(&config, listener.accept().await, &mut server_channels).await.map_err(|e| error!("Error handling stream: {}", e)).ok();
        }
    });

    let mut interval = tokio::time::interval(std::time::Duration::from_millis(800));
    let mut pad = pad::PadState::new();
    pad.connect_device();

    loop {
        tokio::select! {
            _ = interval.tick() => {
                pad.keep_alive();
            }
            pad_req = recv_from_server.recv() => {
                let response = pad.respond(pad_req.unwrap());
                send_to_server.send(response).await.unwrap();
            }
        }
    }
}
