mod config;
mod irc;

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
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio_tls;

use config::{Config, Network};
use irc::Message;

// TODO(jsvana): make NewType for connection key (username:network_name)

type GuardedQueueMap = Arc<Mutex<BTreeMap<String, Sender<Message>>>>;

fn respond_to_ping(message: Message, server_messages: &mut Sender<Message>) -> Result<()> {
    match message.params().last() {
        Some(last) => {
            server_messages.try_send(Message::from_str(&format!("PONG :{}", last))?)?;
            Ok(())
        }
        None => Err(format_err!("PING message has no parameters")),
    }
}

// TODO(jsvana): use this for incoming user connections
/*
async fn write_worker<T>(
    user_reader: ReadHalf<T>,
    mut server_messages: Sender<Message>,
) -> Result<()>
where
    T: AsyncRead,
{
    let server_reader = BufReader::new(user_reader);
    let mut lines = server_reader.lines();
    while let Some(line) = lines.next_line().await? {
        server_messages.try_send(Message::from_str(&line)?)?;
    }

    Ok(())
}
*/

async fn start_workers(_queues: GuardedQueueMap, _user_socket: TcpStream) -> Result<()> {
    // TODO(jsvana): read here
    Ok(())
}

async fn individual_network_read_worker(
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

async fn individual_network_worker(queues: GuardedQueueMap, network: &Network) -> Result<()> {
    let (server_reader, server_writer) = connect_to_network(network).await?;

    // TODO(jsvana): Create MPSC queue and send NICK and USER
    // commands (and PASS if necessary)
    // [2020-01-01T05:02:34Z TRACE bounce] [send] NICK :Guest87
    // [2020-01-01T05:02:34Z TRACE bounce] [send] USER textual 0 * :Textual User

    // TODO(jsvana): make buffer size configurable?
    let (mut server_messages_tx, server_messages_rx) = channel::<Message>(10);

    server_messages_tx.try_send(Message::from_str(&format!("NICK {}", network.alt_nick))?)?;
    server_messages_tx.try_send(Message::from_str(&format!(
        "USER {} 0 * :{}",
        network.username, network.realname
    ))?)?;

    queues.lock().await.insert(
        format!("{}:{}", network.username, network.name),
        server_messages_tx.clone(),
    );

    let (read_result, write_result) = join(
        individual_network_read_worker(server_reader, server_messages_tx.clone()),
        individual_network_write_worker(server_writer, server_messages_rx),
    )
    .await;

    read_result?;
    write_result?;

    Ok(())
}

// TODO(jsvana): take in a guarded map here? That way each connection
// can add its queue Sender and then the server_listener_worker
// can create readers that submit to that queue as necessary
async fn server_connections_worker(queues: GuardedQueueMap, config: &Config) -> Result<()> {
    let mut connections = Vec::new();

    for network in config.networks.iter() {
        connections.push(individual_network_worker(Arc::clone(&queues), network));
    }

    join_all(connections).await;

    Ok(())
}

// TODO(jsvana): take in the guarded map from above here
async fn server_listener_worker(queues: GuardedQueueMap, config: &Config) -> Result<()> {
    let bind_address = config.core.bind_address();
    let mut listener = TcpListener::bind(&bind_address).await?;

    debug!("listening on {}", bind_address);
    loop {
        let (socket, remote_address) = listener.accept().await?;
        debug!("new connection from {}", remote_address);
        start_workers(Arc::clone(&queues), socket).await?;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = Config::from_file("config.toml")?;

    let queues = Arc::new(Mutex::new(BTreeMap::new()));

    // TODO(jsvana): occasionally `server_connections_worker` isn't
    // getting started. Where are you, deadlock?
    /*
    let (connection_manager_result, listener_worker_result) = join(
        server_connections_worker(Arc::clone(&queues), &config),
        server_listener_worker(Arc::clone(&queues), &config),
    )
    .await;

    connection_manager_result?;
    listener_worker_result?;
    */

    server_connections_worker(Arc::clone(&queues), &config).await?;

    Ok(())
}
