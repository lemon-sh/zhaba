use color_eyre::Result;
use serde::Deserialize;
use std::{env, fs, net::SocketAddr, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub log_level: String,
    pub listen: SocketAddr,
    pub image_path: PathBuf,
    pub db: Option<String>,
    pub cookie_secret: String,
    pub whois_server: String,
    pub max_upload_size: usize,
    pub max_post_length: usize,
    pub admins: Vec<Admin>,
}

#[derive(Debug, Deserialize)]
pub struct Admin {
    pub name: String,
    pub password: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let env = env::var("ZHABA_CONFIG");
        let path = env.as_deref().unwrap_or("zhaba.toml");
        let config_str = fs::read_to_string(path)?;
        Ok(toml::from_str(&config_str)?)
    }
}
