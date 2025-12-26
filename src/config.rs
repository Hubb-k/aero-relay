use serde::Deserialize;
use std::collections::HashMap;
use anyhow::Result;

#[derive(Deserialize, Clone, Debug)]
pub struct RelayPair {
    pub name: String,
    pub src_chain: String,
    pub src_rpc: String,
    pub src_channel: String,
    pub src_port: String,      // Matches TOML structure
    pub dst_chain: String,
    pub dst_rpc: String,
    pub dst_channel: String,
    pub dst_port: String,      // Matches TOML structure
    #[serde(default)]
    pub private_key_src: Option<String>,
    #[serde(default)]
    pub private_key_dst: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub relays: Vec<RelayPair>,
    #[serde(default)]
    pub presets: HashMap<String, RelayPair>,
}

impl Config {
    /// Loads configuration from a TOML file.
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}