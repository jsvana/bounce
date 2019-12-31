mod irc;

use std::net::ToSocketAddrs;
//use std::str::FromStr;

use anyhow::Result;
use futures::future::join;
use native_tls::TlsConnector;
use tokio::net::{TcpListener, TcpStream};
use tokio_tls;

//use irc::Message;

async fn read_worker<T, U>(
    mut server_reader: tokio::io::ReadHalf<T>,
    mut user_writer: tokio::io::WriteHalf<U>,
) -> Result<()>
where
    T: tokio::io::AsyncRead,
    U: tokio::io::AsyncWrite,
{
    tokio::io::copy(&mut server_reader, &mut user_writer).await?;
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

async fn start_workers(user_socket: TcpStream) -> Result<()> {
    let addr = "irc.hs.gy:9999".to_socket_addrs().unwrap().next().unwrap();

    let server_socket = TcpStream::connect(&addr).await?;
    let cx = TlsConnector::builder().build().unwrap();
    let cx = tokio_tls::TlsConnector::from(cx);

    let server_socket = cx.connect("irc-west.hs.gy", server_socket).await?;

    let (server_reader, server_writer) = tokio::io::split(server_socket);

    let (user_reader, user_writer) = tokio::io::split(user_socket);

    // TODO(jsvana): Both futures need the writer
    let results = join(
        read_worker(server_reader, user_writer),
        write_worker(user_reader, server_writer),
    )
    .await;

    let (read_result, write_result) = results;
    read_result?;
    write_result?;

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
