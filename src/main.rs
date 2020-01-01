mod config;
mod irc;
mod log_manager;
mod server;

use std::collections::BTreeMap;
//use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
//use futures::channel::mpsc::Sender;
use futures::lock::Mutex;
use log::{debug, error};
use tokio::net::{TcpListener, TcpStream};

use config::Config;
//use irc::Message;
use log_manager::LogManager;

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

async fn start_workers(_queues: server::GuardedQueueMap, _user_socket: TcpStream) -> Result<()> {
    // TODO(jsvana): read here
    debug!("Starting worker now");
    Ok(())
}

// TODO(jsvana): take in the guarded map from above here
async fn server_listener_worker(queues: server::GuardedQueueMap, config: &Config) -> Result<()> {
    let bind_address = config.core.bind_address();
    let mut listener = TcpListener::bind(&bind_address).await?;

    debug!("listening on {}", bind_address);
    loop {
        let (socket, remote_address) = listener.accept().await?;
        debug!("new connection from {}", remote_address);
        let local_queues = Arc::clone(&queues);
        tokio::spawn(async move {
            if let Err(e) = start_workers(local_queues, socket).await {
                error!("Got an error {}", e);
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = Config::from_file("config.toml")?;

    let log_manager = Arc::new(Mutex::new(LogManager::new(&config).await?));

    // This map contains all of the communication queues for servers
    let queues = Arc::new(Mutex::new(BTreeMap::new()));

    let thread_queues = Arc::clone(&queues);
    let thread_config = config.clone();
    tokio::spawn(async move {
        if let Err(e) = server_listener_worker(thread_queues, &thread_config).await {
            error!("Got an error {}", e);
        }
    });

    server::connections_worker(log_manager, queues, &config).await?;

    Ok(())
}
