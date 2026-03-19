use std::path::PathBuf;

pub mod config;
pub mod device;
pub mod profile;

pub const APP_NAME: &'static str = "switchboard";

fn get_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(APP_NAME))
}
