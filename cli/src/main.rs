use std::{io::Write, time::Duration};

// use bluer::{
//     rfcomm::{self, Socket, SocketAddr},
//     Address,
// };

use bluetooth_serial_port_async::BtAddr;
use snafu::Whatever;

#[snafu::report]
#[tokio::main]
async fn main() -> Result<(), Whatever> {
    let image = d30::generate_image("MY LITTLE PONE", 40f32);
    let addr = BtAddr([164, 7, 51, 76, 23, 54]);
    let mut socket =
        bluetooth_serial_port_async::BtSocket::new(bluetooth_serial_port_async::BtProtocol::RFCOMM)
            .unwrap();
    socket.connect(addr).unwrap();
    d30::init_conn(&mut socket);
    let mut output = d30::IMG_PRECURSOR.to_vec();
    output.extend(d30::pack_image(&image));
    socket.write(output.as_slice()).unwrap();
    socket.flush().unwrap();
    Ok(())
}
