use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

use crate::{config::get_config_dir, device::Device};

#[derive(Deserialize)]
pub struct DevicesConfig {
    devices: Vec<DeviceConfig>,
}

#[derive(Deserialize)]
pub struct DeviceConfig {
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub iface: u8,
    pub active: bool,
    pub profile: String,
}

pub struct DeviceInfo {
    pub name: String,
    pub device: Device,
    pub active: bool,
    pub profile: String,
}

pub fn load() -> Result<Vec<DeviceInfo>> {
    if let Some(base_path) = get_config_dir() {
        let path = base_path.join("devices.toml");

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let parsed: DevicesConfig = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        return Ok(parsed
            .devices
            .into_iter()
            .map(|d| DeviceInfo {
                name: d.name,
                device: Device::new(d.vid, d.pid, d.iface),
                active: d.active,
                profile: d.profile,
            })
            .collect());
    }

    anyhow::bail!("Could not open devices.toml");
}
