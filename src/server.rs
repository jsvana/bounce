use std::collections::BTreeMap;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{format_err, Result};
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::future::{join, join_all};
use futures::lock::Mutex;
use futures::StreamExt;
use log::{debug, trace};
use native_tls::TlsConnector;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, BufReader};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_tls;

use super::config::{Config, Network};
use super::irc::Message;
use super::log_manager::LogManager;

pub type GuardedQueueMap = Arc<Mutex<BTreeMap<String, Sender<Message>>>>;

fn respond_to_ping(message: Message, server_messages: &mut Sender<Message>) -> Result<()> {
    match message.params().last() {
        Some(last) => {
            server_messages.try_send(Message::from_str(&format!("PONG :{}", last))?)?;
            Ok(())
        }
        None => Err(format_err!("PING message has no parameters")),
    }
}

async fn individual_network_read_worker(
    config: &Network,
    log_manager: Arc<Mutex<LogManager>>,
    server_reader: Pin<Box<dyn AsyncRead + Unpin>>,
    mut messages: Sender<Message>,
) -> Result<()> {
    let server_reader = BufReader::new(server_reader);
    let mut lines = server_reader.lines();
    while let Some(line) = lines.next_line().await? {
        let message = Message::from_str(&line)?;

        if message.command() == "PING".to_string() {
            respond_to_ping(message, &mut messages)?;
            continue;
        }

        log_manager
            .lock()
            .await
            .add_message(
                &config.username,
                &config.name,
                /*channel=*/ None,
                &message,
            )
            .await?;

        trace!("[recv] {}", message);

        // TODO(jsvana): write messages to log here
    }

    Ok(())
}

// TODO(jsvana): This should be reading from a queue that
// both `read_worker` and incoming client connections write
// to, but then it needs to somehow be exposed to the
// incoming connections. Maybe `ServerConnectionManager`
// should hand out copies of the Sender-side of the queue
// as necessary? Not sure how to reference them, though.
async fn individual_network_write_worker(
    _config: &Network,
    mut server_writer: Pin<Box<dyn AsyncWrite>>,
    mut messages: Receiver<Message>,
) -> Result<()> {
    while let Some(message) = messages.next().await {
        trace!("[send] {}", message);
        server_writer
            .write_all(format!("{}\r\n", message).as_bytes())
            .await?;
    }

    Ok(())
}

async fn individual_network_worker(
    log_manager: Arc<Mutex<LogManager>>,
    queues: GuardedQueueMap,
    network: &Network,
) -> Result<()> {
    let (server_reader, server_writer) = connect_to_network(network).await?;

    // TODO(jsvana): make buffer size configurable?
    let (mut server_messages_tx, server_messages_rx) = channel::<Message>(10);

    server_messages_tx.try_send(Message::from_str(&format!(
        "NICK {}",
        network.nick_choices[0]
    ))?)?;
    server_messages_tx.try_send(Message::from_str(&format!(
        "USER {} 0 * :{}",
        network.username, network.realname
    ))?)?;

    queues.lock().await.insert(
        format!("{}:{}", network.username, network.name),
        server_messages_tx.clone(),
    );

    let (read_result, write_result) = join(
        individual_network_read_worker(
            &network,
            log_manager,
            server_reader,
            server_messages_tx.clone(),
        ),
        individual_network_write_worker(&network, server_writer, server_messages_rx),
    )
    .await;

    read_result?;
    write_result?;

    Ok(())
}

// TODO(jsvana): maybe wrap in a struct?
async fn connect_to_network(
    network: &Network,
) -> Result<(
    Pin<Box<dyn AsyncRead + Unpin>>,
    Pin<Box<dyn AsyncWrite + Unpin>>,
)> {
    let addr = network
        .server
        .address()
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();

    if network.server.ssl {
        // Occasionally we simply hang here without making progress.
        // Not sure why yet.
        let socket = TcpStream::connect(&addr).await?;

        let cx = TlsConnector::builder().build().unwrap();
        let cx = tokio_tls::TlsConnector::from(cx);

        let socket = cx.connect(&network.server.hostname, socket).await?;

        debug!(
            "SSL connection to {} ({}) established",
            network.name,
            network.server.address(),
        );

        let (read_socket, write_socket) = tokio::io::split(socket);

        Ok((
            Pin::new(Box::new(read_socket)),
            Pin::new(Box::new(write_socket)),
        ))
    } else {
        let socket = TcpStream::connect(&addr).await?;

        debug!(
            "unencrypted connection to {} ({}) established",
            network.name,
            network.server.address(),
        );

        let (read_socket, write_socket) = tokio::io::split(socket);

        Ok((
            Pin::new(Box::new(read_socket)),
            Pin::new(Box::new(write_socket)),
        ))
    }
}

pub async fn connections_worker(
    log_manager: Arc<Mutex<LogManager>>,
    queues: GuardedQueueMap,
    config: &Config,
) -> Result<()> {
    let mut connections = Vec::new();

    for network in config.networks.iter() {
        connections.push(individual_network_worker(
            Arc::clone(&log_manager),
            Arc::clone(&queues),
            network,
        ));
    }

    join_all(connections).await;

    Ok(())
}
