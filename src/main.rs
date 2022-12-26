// Setup a tokio server which listens to UNIX socket connections
mod server;
mod config;
use serde::{Serialize, Deserialize};
use postcard::{to_slice, from_bytes};
use tokio::net::UnixListener;

#[tokio::main]
async fn main() {
    config::load_config();
    // Check if file /tmp/hardware.sock exists, if so, delete it
    if std::path::Path::new("/tmp/hardware.sock").exists() {
        std::fs::remove_file("/tmp/hardware.sock").unwrap();
    }
   let mut listener = UnixListener::bind("/tmp/hardware.sock").unwrap();
   loop {
       server::handle_stream(listener.accept().await).await;
  }
}
// #[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
// enum Operation {
//     KeepAlive,
//     SabertoothWrite(u8, u8),
//     SmartelexWrite(u8, [u8; 5]),
//     EncoderRead(u8, u8),
//     PwmWrite(u8, u16),
// }
// fn main() {
//     let mut port = serialport::new("/dev/ttyACM0", 9600).open().unwrap();
//     port.set_timeout(std::time::Duration::from_millis(1000)).unwrap();
//     let mut buf = [0u8; 64];
//     let op = Operation::SmartelexWrite(4, [0x2A, 0x08, 0xFF, 0xFF, 0x23]);
//     let coded = to_slice(&op, &mut buf).unwrap();
//     port.write(&coded).unwrap();
//     println!("Written bytes: {:?}", coded);
//     let op = Operation::SabertoothWrite(3, 192);
//     let coded = to_slice(&op, &mut buf).unwrap();
//     port.write(&coded).unwrap();
//     println!("Written bytes: {:?}", coded);
// }
