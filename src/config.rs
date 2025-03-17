use log::warn;
use serde::Deserialize;
use std::{
    fs::File,
    io::{Read, Write},
};

use crate::error::AppError;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub world: WorldConfig,
}

// Hmm hmm hmm hmm... the great, iconic, symbol of nobility. My sibilantic friend, ServerSonfig.
// Your hour has passed and
#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub name: String,
    pub motd: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WorldConfig {
    pub world: String,
    pub size_x: i16,
    pub size_y: i16,
    pub size_z: i16,
}

impl Config {
    pub fn load() -> Result<Self, AppError> {
        // Load the config file
        let mut config_file = match File::open("config.toml") {
            Ok(result) => result,
            Err(_) => {
                const CONFIG_FILE_DATA: &str = r#"[server]
name = "mcrizzledizzle default"
motd = "For shits and giggles"
port = 25565

[world]
world = "world.wrld" # Custom world type, not interchangable
# Generation parameters, when a world is read these are ignored
size_x = 64
size_y = 32
size_z = 64
"#;

                warn!("No config file was present! Generating one now.");
                let mut config_file = File::create("config.toml")?;
                config_file.write_all(CONFIG_FILE_DATA.as_bytes())?;
                File::open("config.toml").expect("Failed to create config.toml")
            }
        };

        let mut config_data = String::new();
        config_file
            .read_to_string(&mut config_data)
            .expect("Failed to read config file");

        Ok(toml::from_str(&config_data)?)
    }
}
