use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

const CONFIG_DIR: &str = ".strata";
const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub xpub_mainnet: Option<String>,
    pub xpub_testnet: Option<String>,
    #[serde(default)]
    pub address_index_mainnet: u32,
    #[serde(default)]
    pub address_index_testnet: u32,
}

fn get_config_path() -> Result<PathBuf, Box<dyn Error>> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home.join(CONFIG_DIR).join(CONFIG_FILE))
}

fn get_config_dir() -> Result<PathBuf, Box<dyn Error>> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home.join(CONFIG_DIR))
}

pub fn load_config() -> Result<Config, Box<dyn Error>> {
    let path = get_config_path()?;

    if !path.exists() {
        return Ok(Config::default());
    }

    let contents = fs::read_to_string(&path)?;
    let config: Config = serde_json::from_str(&contents)?;
    Ok(config)
}

pub fn save_config(config: &Config) -> Result<(), Box<dyn Error>> {
    let dir = get_config_dir()?;
    let path = get_config_path()?;

    // Create directory if it doesn't exist
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    let contents = serde_json::to_string_pretty(config)?;
    fs::write(&path, contents)?;
    Ok(())
}
