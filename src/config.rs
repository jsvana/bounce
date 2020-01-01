use anyhow::{format_err, Result};
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Core {
    pub bind_hostname: String,
    pub bind_port: u16,
}

impl Core {
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.bind_hostname, self.bind_port)
    }
}

#[derive(Debug, Deserialize)]
pub struct NetworkServer {
    pub hostname: String,
    pub port: u16,
    pub ssl: bool,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Network {
    pub name: String,
    pub nick: String,
    pub alt_nick: String,
    pub ident: String,
    pub realname: String,

    pub server: NetworkServer,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub core: Core,
    pub networks: Vec<Network>,
}

impl Config {
    pub fn from_file(filename: &str) -> Result<Self> {
        toml::from_str(&std::fs::read_to_string(filename)?)
            .map_err(|e| format_err!("Failed to read configuration: {}", e))
    }
}
