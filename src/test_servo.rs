use postcard::{from_bytes, to_slice};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
enum Operation {
    KeepAlive,
    SabertoothWrite(u8, u8),
    SmartelexWrite(u8, [u8; 5]),
    EncoderRead,
    PwmWrite(u8, u16),
}

fn main() {
    let mut port = serialport::new("/dev/ttyACM0", 9600).open().unwrap();
    port.set_timeout(std::time::Duration::from_millis(1000)).unwrap();
    let mut buf = [0u8; 1024];

    loop {
        let op = Operation::PwmWrite(0, 200u16);
        let coded = to_slice(&op, &mut buf).unwrap();
        let _ = port.write(coded).unwrap();
        println!("Written bytes: {:?}", coded);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
