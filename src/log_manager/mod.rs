//! Manages IRC logs.
//!
//! Structure:
//!   <username>/
//!     <server:hostport>/
//!       <channel>/
//!         log

// TODO(jsvana): Maybe store hourly offsets in an index
// file to make replay easier?

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio::io::AsyncWriteExt;

use super::config::Config;
use super::irc::Message;

const LOGFILE_STR: &str = "LOG";

pub struct LogManager {
    base_path: PathBuf,
    file_handles: BTreeMap<PathBuf, File>,
}

/*
trait IrcLog {
    async fn add_message(
        user: &User,
        server: &Server,
        channel: &Channel,
        message: &Message,
    ) -> Result<()>;

    async fn replay_messages_from_time(
        user: &User,
        server: &Server,
        channel: &Channel,
        start_time: &Instant,
    ) -> Result<Vec<Message>>;
}
*/

impl LogManager {
    pub async fn new(config: &Config) -> Result<Self> {
        create_dir_all(&config.log.base_path).await?;

        Ok(Self {
            base_path: config.log.base_path.clone(),
            file_handles: BTreeMap::new(),
        })
    }

    fn path_from_params(&self, user: &str, server: &str, channel: &str) -> PathBuf {
        vec![
            &self.base_path,
            &PathBuf::from(user),
            &PathBuf::from(server),
            &PathBuf::from(channel),
        ]
        .iter()
        .collect()
    }

    pub async fn add_message(
        &mut self,
        user: &str,
        server: &str,
        channel: &str,
        message: &Message,
    ) -> Result<()> {
        let dir_path = self.path_from_params(user, server, channel);

        let file_path: PathBuf = vec![&dir_path, &PathBuf::from(LOGFILE_STR)]
            .iter()
            .collect();

        if !self.file_handles.contains_key(&file_path) {
            create_dir_all(&dir_path).await?;

            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&file_path)
                .await?;

            self.file_handles.insert(file_path.clone(), file);
        }

        self.file_handles
            .get_mut(&file_path)
            .unwrap()
            .write(format!("{}\r\n", message).as_bytes())
            .await?;

        Ok(())
    }
}
