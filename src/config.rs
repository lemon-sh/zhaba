use serde::Deserialize;
use std::{env, fs, net::SocketAddr};
use color_eyre::Result;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub log_level: String,
    pub listen: SocketAddr,
    pub image_path: String,
    pub db: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let env = env::var("ZHABA_CONFIG");
        let path = env.as_deref().unwrap_or("zhaba.toml");
        let config_str = fs::read_to_string(path)?;
        Ok(toml::from_str(&config_str)?)
    }
}
