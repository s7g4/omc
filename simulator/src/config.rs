use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub satellite_id: Uuid,
    pub backend_url: String,
    pub tick_interval_ms: u64,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config: Config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    pub fn save_default<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let default_config = Config {
            satellite_id: Uuid::new_v4(),
            backend_url: "http://127.0.0.1:8081/api/v1/telemetry".to_string(),
            tick_interval_ms: 1000,
        };
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &default_config)?;
        Ok(default_config)
    }
}
