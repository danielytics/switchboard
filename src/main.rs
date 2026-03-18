use std::io::{self, Write};
use std::time::Duration;

pub mod device;
pub mod keys;

use crate::device::{Device, OnKeyResult, PollResult, PollSettings};

fn main() {
    println!("USB HID Device Selector");
    println!("=======================\n");

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

    device.read_key_loop(Duration::from_millis(100), &mut |ev| {
        println!("{}", ev);

        if ev.contains_key(exit_key.keys[0]) {
            OnKeyResult::Break
        } else {
            OnKeyResult::Continue
        }
    });

    drop(device);
}
