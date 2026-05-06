use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub cells: std::collections::HashMap<String, CellConfig>,
    #[serde(default)]
    pub timers: std::collections::HashMap<String, TimerConfig>,
    #[serde(default)]
    pub layers: Vec<LayerConfig>,
}

#[derive(Deserialize, Serialize)]
pub struct CellConfig {
    pub min: i64,
    pub max: i64,
    #[serde(default)]
    pub default: Option<i64>,
    #[serde(default)]
    pub wrap: bool,
    #[serde(default)]
    pub settle_ms: Option<u64>,
    #[serde(default)]
    pub mapping: std::collections::HashMap<i64, String>,
}

#[derive(Deserialize, Serialize)]
pub struct TimerConfig {
    pub timeout_ms: u64,
    #[serde(default)]
    pub repeat: bool,
}

#[derive(Deserialize, Serialize)]
pub struct LayerConfig {
    pub name: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub exclusive_group: Option<String>,
}
