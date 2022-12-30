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
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct EncoderValues {
    values: [i32; 5],
}
fn main() {
    let mut port = serialport::new("/dev/ttyACM0", 9600).open().unwrap();
    port.set_timeout(std::time::Duration::from_millis(1000))
        .unwrap();
    let mut buf = [0u8; 64];
    loop {
        let op = Operation::EncoderRead;
        let coded = to_slice(&op, &mut buf).unwrap();
        let _ = port.write(coded).unwrap();
        println!("Written bytes: {:?}", coded);
        let read = port.read(&mut buf).unwrap();
        let EncoderValues { values } = from_bytes(&buf[..read]).unwrap();
        println!("Read: {:?}", values);
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
