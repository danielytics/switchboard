use regex::Regex;
use std::{path::PathBuf, sync::LazyLock};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Device {
        #[command(subcommand)]
        action: DeviceCommands,
    },
}

#[derive(Subcommand)]
enum DeviceCommands {
    Add {
        #[arg(long, value_name = "NAME")]
        name: Option<String>,

        #[arg(long, value_name = "VID:PID")]
        id: Option<String>,
    },
    List,
    Remove {
        name: String,
    },
}

pub struct DeviceId {
    pub vid: u16,
    pub pid: u16,
}

static VID_PID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9a-fA-F]{4}):([0-9a-fA-F]{4})$").unwrap());

impl TryFrom<String> for DeviceId {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let groups = VID_PID_RE.captures(&value).with_context(|| {
            format!("'{}' must be exactly 4 hex digits (e.g. 046d:c52b)", value)
        })?;

        let vid = u16::from_str_radix(groups.get(1).unwrap().as_str(), 16)?;
        let pid = u16::from_str_radix(groups.get(2).unwrap().as_str(), 16)?;
        Ok(DeviceId { vid, pid })
    }
}

pub enum Action {
    Run,
    AddDevice {
        name: Option<String>,
        id: Option<DeviceId>,
    },
    ListDevices,
    RemoveDevice {
        name: String,
    },
}

impl TryFrom<Cli> for Action {
    type Error = anyhow::Error;

    fn try_from(cli: Cli) -> Result<Self, Self::Error> {
        match cli.command {
            Some(Commands::Device { action }) => match action {
                DeviceCommands::Add { name, id } => {
                    let id = match id {
                        Some(s) => Some(DeviceId::try_from(s)?),
                        None => None,
                    };
                    Ok(Action::AddDevice { name, id })
                }
                DeviceCommands::List => Ok(Action::ListDevices),
                DeviceCommands::Remove { name } => Ok(Action::RemoveDevice { name }),
            },
            None => Ok(Action::Run),
        }
    }
}

pub async fn exec() -> Result<Action> {
    let cli = Cli::parse();
    let action = Action::try_from(cli)?;
    Ok(action)
}
