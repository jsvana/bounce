use std::path::PathBuf;

use anyhow::{format_err, Result};
use serde_derive::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Core {
    pub bind_hostname: String,
    pub bind_port: u16,
}

impl Core {
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.bind_hostname, self.bind_port)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Log {
    pub base_path: PathBuf,
}

fn default_port() -> u16 {
    6667
}

fn default_ssl() -> bool {
    false
}

#[derive(Clone, Debug, Deserialize)]
pub struct NetworkServer {
    pub hostname: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_ssl")]
    pub ssl: bool,
    pub password: Option<String>,
}

impl NetworkServer {
    pub fn address(&self) -> String {
        format!("{}:{}", self.hostname, self.port)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Network {
    pub name: String,
    pub nick_choices: Vec<String>,
    pub username: String,
    pub realname: String,

    pub server: NetworkServer,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub core: Core,
    pub log: Log,
    pub networks: Vec<Network>,
}

impl Config {
    pub fn from_file(filename: &str) -> Result<Self> {
        let config: Self = toml::from_str(&std::fs::read_to_string(filename)?)
            .map_err(|e| format_err!("Failed to read configuration: {}", e))?;

        for network in config.networks.iter() {
            if network.nick_choices.len() == 0 {
                return Err(format_err!(
                    "Must specify at least one nick for network \"{}\" ({})",
                    network.name,
                    network.server.address()
                ));
            }
        }

        Ok(config)
    }
}
