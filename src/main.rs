mod irc;

use std::net::ToSocketAddrs;
use std::str::FromStr;

use anyhow::{format_err, Result};
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::future::join3;
use futures::StreamExt;
use native_tls::TlsConnector;
use tokio::io::AsyncBufReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio_tls;

use irc::Message;

async fn server_writer_worker<T>(
    mut server_writer: tokio::io::WriteHalf<T>,
    mut messages: Receiver<Message>,
) -> Result<()>
where
    T: tokio::io::AsyncWrite,
{
    while let Some(message) = messages.next().await {
        println!("SENDING {}", message);
        server_writer
            .write_all(format!("{}\r\n", message).as_bytes())
            .await?;
    }

    Ok(())
}

fn respond_to_ping(message: Message, server_messages: &mut Sender<Message>) -> Result<()> {
    match &message.params() {
        Some(params) => match params.last() {
            Some(last) => {
                server_messages.try_send(Message::from_str(&format!("PONG :{}", last))?)?;
                Ok(())
            }
            None => Err(format_err!("PING message has no parameters")),
        },
        None => Err(format_err!("PING message has no parameters")),
    }
}

async fn read_worker<T, U>(
    server_reader: tokio::io::ReadHalf<T>,
    mut server_messages: Sender<Message>,
    mut user_writer: tokio::io::WriteHalf<U>,
) -> Result<()>
where
    T: tokio::io::AsyncRead,
    U: tokio::io::AsyncWrite,
{
    let server_reader = tokio::io::BufReader::new(server_reader);
    let mut lines = server_reader.lines();
    while let Some(line) = lines.next_line().await? {
        let message = Message::from_str(&line)?;

        if message.command() == "PING".to_string() {
            respond_to_ping(message, &mut server_messages)?;
            continue;
        }

        println!("Read {}", message);

        user_writer
            .write_all(format!("{}\r\n", message).as_bytes())
            .await?;
    }

    //tokio::io::copy(&mut server_reader, &mut user_writer).await?;

    Ok(())
}

async fn write_worker<T>(
    user_reader: tokio::io::ReadHalf<T>,
    mut server_messages: Sender<Message>,
) -> Result<()>
where
    T: tokio::io::AsyncRead,
{
    let server_reader = tokio::io::BufReader::new(user_reader);
    let mut lines = server_reader.lines();
    while let Some(line) = lines.next_line().await? {
        server_messages.try_send(Message::from_str(&line)?)?;
    }

    Ok(())
}

async fn start_workers(user_socket: TcpStream) -> Result<()> {
    let (server_messages_tx, server_messages_rx) = channel(10);

    let addr = "irc.hs.gy:9999".to_socket_addrs().unwrap().next().unwrap();

    let server_socket = TcpStream::connect(&addr).await?;
    let cx = TlsConnector::builder().build().unwrap();
    let cx = tokio_tls::TlsConnector::from(cx);

    let server_socket = cx.connect("irc-west.hs.gy", server_socket).await?;

    let (server_reader, server_writer) = tokio::io::split(server_socket);

    let (user_reader, user_writer) = tokio::io::split(user_socket);

    // The server-side threads should be started on startup

    // TODO(jsvana): Both futures need the writer
    let results = join3(
        read_worker(server_reader, server_messages_tx.clone(), user_writer),
        write_worker(user_reader, server_messages_tx.clone()),
        server_writer_worker(server_writer, server_messages_rx),
    )
    .await;

    let (read_result, write_result, server_writer_result) = results;
    read_result?;
    write_result?;
    server_writer_result?;

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
