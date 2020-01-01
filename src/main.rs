mod config;
mod irc;
mod server;

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Result;
//use futures::channel::mpsc::Sender;
use futures::lock::Mutex;
//use log::debug;
//use tokio::net::{TcpListener, TcpStream};

use config::Config;
//use irc::Message;

const FOO: usize = 42;

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

/*
async fn start_workers(_queues: server::GuardedQueueMap, _user_socket: TcpStream) -> Result<()> {
    // TODO(jsvana): read here
    Ok(())
}
*/

fn hello() {
    println!("Hi");
}

// TODO(jsvana): take in the guarded map from above here
/*
async fn server_listener_worker(queues: server::GuardedQueueMap, config: &Config) -> Result<()> {
    let bind_address = config.core.bind_address();
    let mut listener = TcpListener::bind(&bind_address).await?;

    debug!("listening on {}", bind_address);
    loop {
        let (socket, remote_address) = listener.accept().await?;
        debug!("new connection from {}", remote_address);
        start_workers(Arc::clone(&queues), socket).await?;
    }
}
*/

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    hello();

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

    server::connections_worker(Arc::clone(&queues), &config).await?;

    Ok(())
}
