mod irc;

use std::net::{SocketAddr, ToSocketAddrs};
//use std::str::FromStr;

use anyhow::Result;
use futures::future::join_all;
use native_tls::TlsConnector;
use tokio::io::AsyncBufReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio_tls;
use tokio_tls::TlsStream;

//use irc::Message;

const IRC_MESSAGE_LENGTH: usize = 512;

// TODO(jsvana): figure this out...apparently implementing AsyncRead
// means introducing `unsafe` :(
/*
enum StreamEither {
    Left(TcpStream),
    Right(TlsStream<TcpStream>),
}
*/

struct IrcServer {
    name: String,
    host_port: SocketAddr,
    // password: String,
}

async fn read_worker<T, U>(
    _server_reader: tokio::io::ReadHalf<T>,
    _user_writer: tokio::io::WriteHalf<U>,
) -> Result<()>
where
    T: tokio::io::AsyncRead,
    U: tokio::io::AsyncWrite,
{
    Ok(())
}

async fn write_worker<T, U>(
    mut user_reader: tokio::io::ReadHalf<T>,
    mut server_writer: tokio::io::WriteHalf<U>,
) -> Result<()>
where
    T: tokio::io::AsyncRead,
    U: tokio::io::AsyncWrite,
{
    tokio::io::copy(&mut user_reader, &mut server_writer).await?;

    Ok(())
}

async fn start_workers(mut user_socket: TcpStream) -> Result<()> {
    let addr = "irc.hs.gy:9999".to_socket_addrs().unwrap().next().unwrap();

    let server_socket = TcpStream::connect(&addr).await?;
    let cx = TlsConnector::builder().build().unwrap();
    let cx = tokio_tls::TlsConnector::from(cx);

    let server_socket = cx.connect("irc-west.hs.gy", server_socket).await?;

    let (server_reader, mut server_writer) = tokio::io::split(server_socket);

    let (mut user_reader, user_writer) = tokio::io::split(user_socket);

    // TODO(jsvana): figure this out. These two together can't be awaited
    let futures = vec![
        // TODO(jsvana): Both futures need the writer
        write_worker(user_reader, server_writer),
        // This fails to compile:
        /*
         * error[E0308]: mismatched types
         *   --> src/main.rs:75:9
         *    |
         * 75 |         read_worker(server_reader, user_writer),
         *    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected opaque type, found a different opaque type
         *    |
         *    = note: expected type `impl core::future::future::Future` (opaque type at <src/main.rs:48:6>)
         *               found type `impl core::future::future::Future` (opaque type at <src/main.rs:37:6>)
         *    = note: distinct uses of `impl Trait` result in different opaque types
         */
        read_worker(server_reader, user_writer),
    ];

    for result in join_all(futures).await {
        result?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut listener = TcpListener::bind("127.0.0.1:49654").await?;

    println!("Listening on 49654");
    loop {
        let (socket, _) = listener.accept().await?;
        println!("Got new connection");
        start_workers(socket).await?;
    }
}
