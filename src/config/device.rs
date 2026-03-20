use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::{config::get_config_dir, device::Device};

#[derive(Deserialize, Serialize)]
pub struct DevicesConfig {
    devices: Vec<DeviceConfig>,
}

#[derive(Deserialize, Serialize)]
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

impl From<DeviceConfig> for DeviceInfo {
    fn from(value: DeviceConfig) -> Self {
        Self {
            name: value.name,
            device: Device::new(value.vid, value.pid, value.iface),
            active: value.active,
            profile: value.profile,
        }
    }
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
            .map(|d| DeviceInfo::from(d))
            .collect());
    }

    anyhow::bail!("Could not open devices.toml");
}

pub fn save(config: Vec<DeviceInfo>) -> Result<()> {
    if let Some(base_path) = get_config_dir() {
        let path = base_path.join("devices.toml");

        let devices = config
            .into_iter()
            .map(|d| DeviceConfig {
                name: d.name,
                vid: d.device.vid,
                pid: d.device.pid,
                iface: d.device.iface,
                active: d.active,
                profile: d.profile,
            })
            .collect();

        let contents = toml::to_string(&DevicesConfig { devices })
            .with_context(|| format!("Could not serialize devices"))?;

        fs::write(&path, contents)
            .with_context(|| format!("Failed to write {}", path.display()))?;

        return Ok(());
    }

    anyhow::bail!("Could not open devices.toml");
}
