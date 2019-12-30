mod irc;

use std::io::ErrorKind::TimedOut;
use std::net::ToSocketAddrs;
use std::time::Duration;

use anyhow::{format_err, Result};
use native_tls::TlsConnector;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_io_timeout::TimeoutStream;
use tokio_tls;

const IRC_MESSAGE_LENGTH: usize = 512;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "irc.hs.gy:9999".to_socket_addrs().unwrap().next().unwrap();

    let socket = TcpStream::connect(&addr).await?;
    let cx = TlsConnector::builder().build().unwrap();
    let cx = tokio_tls::TlsConnector::from(cx);

    let socket = cx.connect("irc-west.hs.gy", socket).await?;
    let mut socket = TimeoutStream::new(socket);
    socket.set_read_timeout(Some(Duration::from_secs(1)));

    loop {
        let mut buffer = [0; IRC_MESSAGE_LENGTH];

        match socket.read(&mut buffer).await {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    println!("Socket closed");
                    break;
                }
                println!("Read: \"{}\"", std::str::from_utf8(&buffer[..])?);
            }
            Err(e) if e.kind() == TimedOut => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => {
                return Err(format_err!("Failed to read from socket"));
            }
        }
    }

    Ok(())
}
