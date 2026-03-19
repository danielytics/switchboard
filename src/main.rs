use std::io::{self, Write};
use std::time::Duration;

pub mod cli;
pub mod config;
pub mod device;
pub mod keys;

use crate::device::{Device, PollResult, PollSettings};

#[tokio::main]
async fn main() {
    match cli::exec().await {
        Ok(action) => match action {
            cli::Action::AddDevice { name, id: _ } => {
                println!("Poll for new device");
                if let Some(name) = name {
                    println!("Set name to: {}", name);
                }
            }
            cli::Action::ListDevices => {
                println!("List configured devices")
            }
            cli::Action::RemoveDevice { name } => {
                println!("Remove device: {}", name);
            }
            cli::Action::Run => {
                println!("Open active devcies and run loop");
                run().await;
            }
        },
        Err(error) => {
            println!("Error: {}", error);
        }
    }
}

async fn run() {
    println!("USB HID Device Selector");
    println!("=======================\n");

    match config::device::load() {
        Ok(devices_config) => {
            for config in devices_config {
                if config.active {
                    let device =
                        Device::new(config.device.vid, config.device.pid, config.device.iface);
                    println!(
                        "{} -- Active: {}, Profile: {}, Name: {}",
                        device, config.active, config.profile, config.name
                    );
                }
            }
        }
        Err(error) => {
            println!("Error: {}", error)
        }
    }

    let (device, exit_key) = match Device::poll(
        PollSettings::default()
            .with_timeout(Duration::from_secs(5))
            .with_delay(Duration::from_millis(1))
            .with_scan_time(Duration::from_millis(10)),
    ) {
        PollResult::None => {
            println!("No devices found");
            return;
        }
        PollResult::Timeout => {
            println!("No key pressed, exiting");
            return;
        }
        PollResult::Device(device, event) => (device, event),
    };

    // Get name
    println!("\nDetected input:");
    println!("  Name:      {}", device.product_string());
    println!(
        "  VID:PID/I: {:04x}:{:04x}/{}",
        device.info.vid, device.info.pid, device.info.iface
    );
    println!("  Key:       {}", exit_key.to_string());
    println!();

    println!("WARNING: This will exclusively capture the device.");
    println!("Make sure you have another way to control your system!\n");

    print!("Use this device? [y/N] ");
    io::stdout().flush().unwrap();

    unsafe {
        libc::tcflush(0, libc::TCIFLUSH);
    }

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let answer = input.trim().chars().next().unwrap_or('n');
    if answer != 'y' && answer != 'Y' {
        println!("Cancelled");
        return;
    }

    println!("Listening. Press the SAME key again to exit.\n");

    let (mut rx, stop_device, thread_handle) = device.start(Duration::from_millis(100));

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\nCtrl-C received!");
                stop_device();
                break;
            }
            maybe_ev = rx.recv() => {
                if let Some(ev) = maybe_ev {
                    println!("{}", ev);
                    if ev.contains(exit_key) {
                        stop_device();
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    let _ = thread_handle.join();
}
