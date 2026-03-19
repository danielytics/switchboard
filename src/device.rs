use rusb::{Context, DeviceDescriptor, DeviceHandle, Result, UsbContext};
use std::{
    cell::RefCell,
    fmt::Display,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use usb_ids::{self, FromId};

use crate::keys::{KeyEvent, KeyParser};

const HID_CLASS: u8 = 0x03;
// const DEVICE_TO_HOST: u8 = 0xA1; // 10100001b: Device-to-Host, Class, Interface
const HOST_TO_DEVICE: u8 = 0x21; // 00100001b: Host-to-Device, Class, Interface
// const GET_IDLE: u8 = 0x02;
// const GET_PROTOCOL: u8 = 0x03;
const SET_IDLE: u8 = 0x0A;
const SET_PROTOCOL: u8 = 0x0B;

#[derive(Clone)]
pub struct Device {
    pub vid: u16,
    pub pid: u16,
    pub iface: u8,
    pub vendor_name: String,
    pub product_name: String,
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04x}:{:04x}/{}", self.vid, self.pid, self.iface)
    }
}

pub struct PollSettings {
    /// How long to poll for
    timeout: Duration,
    /// How long to wait between each polling iteration
    delay: Duration,
    /// How long te scan each individual device for each iteration
    scan_time: Duration,
}

impl Default for PollSettings {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            delay: Duration::from_millis(1),
            scan_time: Duration::from_millis(10),
        }
    }
}

impl PollSettings {
    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self { timeout, ..self }
    }

    pub fn with_delay(self, delay: Duration) -> Self {
        Self { delay, ..self }
    }

    pub fn with_scan_time(self, scan_time: Duration) -> Self {
        Self { scan_time, ..self }
    }
}

impl Device {
    pub fn new(vid: u16, pid: u16, iface: u8) -> Self {
        let (vendor_name, product_name) = Self::find_static_device_details(vid, pid);
        Self {
            vid,
            pid,
            iface,
            vendor_name: vendor_name.to_string(),
            product_name: product_name.to_string(),
        }
    }

    fn find_static_device_details(vid: u16, pid: u16) -> (&'static str, &'static str) {
        let vendor_name = match usb_ids::Vendor::from_id(vid) {
            Some(vendor) => vendor.name(),
            None => "Unknown vendor",
        };

        let product_name = match usb_ids::Device::from_vid_pid(vid, pid) {
            Some(product) => product.name(),
            None => "Unknown product",
        };

        (vendor_name, product_name)
    }

    pub fn list() -> Vec<Device> {
        let context = Context::new().unwrap();

        context
            .devices()
            .unwrap()
            .iter()
            .flat_map(|device| {
                let dev_desc = device.device_descriptor().unwrap();
                let vid = dev_desc.vendor_id();
                let pid = dev_desc.product_id();
                let (vendor_name, product_name) = Self::find_static_device_details(vid, pid);

                let mut interfaces: Vec<Device> = Vec::new();
                let config = device.active_config_descriptor().ok()?;
                for interface in config.interfaces() {
                    for iface_desc in interface.descriptors() {
                        if iface_desc.class_code() == HID_CLASS {
                            let is_keyboard =
                                iface_desc.sub_class_code() == 1 && iface_desc.protocol_code() == 1;

                            // You can choose to ONLY add keyboards, or prioritize them
                            if is_keyboard {
                                interfaces.push(Device {
                                    vid,
                                    pid,
                                    iface: iface_desc.interface_number(),
                                    vendor_name: vendor_name.to_string(),
                                    product_name: product_name.to_string(),
                                })
                            }
                        }
                    }
                }
                Some(interfaces)
            })
            .flatten()
            .collect()
    }

    pub fn poll(settings: PollSettings) -> PollResult {
        let devices = Device::list();

        if devices.is_empty() {
            return PollResult::None;
        }

        // let mut seen = std::collections::HashSet::new();

        // Open all for polling
        let mut opened: Vec<_> = devices
            .into_iter()
            .filter(|info| !info.vendor_name.contains("Kinesis"))
            // .filter(|info| seen.insert((info.vid, info.pid, info.iface)))
            .filter_map(|info| info.open().ok())
            .collect();

        if opened.is_empty() {
            return PollResult::None;
        }

        let mut detected_device = None;

        let mut buf = vec![0u8; 64];
        let start = std::time::Instant::now();
        'outer: while start.elapsed() < settings.timeout {
            let indices: Vec<_> = opened.iter().enumerate().map(|(i, _)| i).collect();
            for index in indices {
                let device = &opened[index];
                let slice = &mut buf[0..device.max_packet_size];
                if let Some(ev) = device.read_key(slice, settings.scan_time) {
                    if !ev.empty() {
                        let device = opened.swap_remove(index);
                        detected_device = Some((device, ev));
                        break 'outer;
                    }
                } else {
                }
            }

            thread::sleep(settings.delay);
        }

        // Release all polling handles (reattaches drivers)
        drop(opened);

        match detected_device {
            Some((instance, event)) => {
                // Read one more to make sure the buffer is cleared
                let slice = &mut buf[0..instance.max_packet_size];
                loop {
                    if let Some(ev) = instance.read_key(slice, settings.scan_time) {
                        println!("Got: {}", ev);
                    } else {
                        println!("Not got");
                        break;
                    }
                }
                // Return the polled device
                PollResult::Device(instance, event)
            }
            None => PollResult::None,
        }
    }

    pub fn open(&self) -> rusb::Result<DeviceInstance> {
        DeviceInstance::new(&self, None)
    }
}

pub enum PollResult {
    Device(DeviceInstance, KeyEvent),
    Timeout,
    None,
}

pub struct DeviceInstance {
    pub handle: DeviceHandle<Context>,
    pub info: Device,
    pub descriptor: DeviceDescriptor,
    pub endpoint: u8,
    pub max_packet_size: usize,
    had_driver: bool,
    original_protocol: Option<u8>,
    original_idle: Option<u8>,

    cached_manufacturer_string: RefCell<Option<String>>,
    cached_product_string: RefCell<Option<String>>,

    listening: Arc<AtomicBool>,
}

impl Drop for DeviceInstance {
    fn drop(&mut self) {
        let iface = self.info.iface;

        // Restore the protocol if we changed it
        if let Some(protocol) = self.original_protocol {
            let _ = self.handle.write_control(
                HOST_TO_DEVICE,
                SET_PROTOCOL,
                protocol as u16, // Restore original value (usually 1)
                iface as u16,
                &[],
                Duration::from_millis(500),
            );
        }

        // Restore the idle setting
        if let Some(idle) = self.original_idle {
            let _ = self
                .handle
                .write_control(
                    HOST_TO_DEVICE,
                    SET_IDLE,
                    (idle as u16) << 8,
                    iface as u16,
                    &[],
                    Duration::from_millis(1000),
                )
                .ok();
        }

        // Release interface
        let _ = self.handle.release_interface(iface);

        // Re-attach kernel driver
        if self.had_driver {
            let _ = self.handle.attach_kernel_driver(iface);
        }
    }
}

impl DeviceInstance {
    fn new(
        info: &Device,
        device: Option<(rusb::Device<Context>, DeviceDescriptor)>,
    ) -> rusb::Result<DeviceInstance> {
        let vid = info.vid;
        let pid = info.pid;
        let iface = info.iface;

        let context = Context::new()?;

        let (device, descriptor) = if let Some(dev_desc) = device {
            dev_desc
        } else {
            find_device(context, vid, pid)?
        };

        let handle = device.open()?;

        handle.set_active_configuration(1).ok();

        let had_driver = handle.kernel_driver_active(iface).unwrap_or(false);
        if had_driver {
            handle.detach_kernel_driver(iface)?;
        }

        handle.claim_interface(iface)?;

        // Find endpoint
        let config = device.active_config_descriptor()?;
        let mut endpoint = 0x81;
        let mut max_packet_size: usize = 64;
        for interface in config.interfaces() {
            for desc in interface.descriptors() {
                if desc.interface_number() == iface {
                    for ep in desc.endpoint_descriptors() {
                        if ep.transfer_type() == rusb::TransferType::Interrupt
                            && ep.direction() == rusb::Direction::In
                        {
                            endpoint = ep.address();
                            max_packet_size = ep.max_packet_size() as usize;
                            break;
                        }
                    }
                }
            }
        }

        // Reset endpoint
        handle.clear_halt(endpoint).ok();

        // Get original protocol so that we can restore it on drop
        // let mut proto_buf = [0u8; 1];
        // let original_protocol = match handle.read_control(
        //     DEVICE_TO_HOST,
        //     GET_PROTOCOL,
        //     0,            // Value (not used for GET)
        //     iface as u16, // Index (Interface number)
        //     &mut proto_buf,
        //     Duration::from_millis(100),
        // ) {
        //     Ok(1) => Some(proto_buf[0]),
        //     _ => None, // Device might not support GET_PROTOCOL
        // };

        // Force boot protocol
        // let _ = handle.write_control(
        //     HOST_TO_DEVICE,
        //     SET_PROTOCOL,
        //     0, // Value: 0 for Boot Protocol
        //     iface as u16,
        //     &[],
        //     Duration::from_millis(100),
        // );

        // let original_idle = match handle.read_control(
        //     DEVICE_TO_HOST,
        //     GET_IDLE,
        //     0,            // Value (not used for GET)
        //     iface as u16, // Index (Interface number)
        //     &mut proto_buf,
        //     Duration::from_millis(100),
        // ) {
        //     Ok(1) => Some(proto_buf[0]),
        //     _ => None, // Device might not support GET_PROTOCOL
        // };
        // handle
        //     .write_control(
        //         HOST_TO_DEVICE,
        //         SET_IDLE,
        //         0,
        //         iface as u16,
        //         &[],
        //         Duration::from_millis(1000),
        //     )
        //     .ok();

        Ok(DeviceInstance {
            handle,
            info: info.clone(),
            descriptor,
            had_driver,
            endpoint,
            original_protocol: None,
            original_idle: None,
            max_packet_size,
            cached_manufacturer_string: RefCell::new(None),
            cached_product_string: RefCell::new(None),
            listening: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn manufacturer_string(&self) -> String {
        let mut cache = self.cached_manufacturer_string.borrow_mut();
        match cache.clone() {
            Some(text) => text.clone(),
            None => {
                let timeout = Duration::from_secs(1);
                let none = "<none>".to_string();
                let manufacturer_string = match self.handle.read_languages(timeout) {
                    Ok(languages) => {
                        let language = languages[0];
                        clean_string(
                            self.handle.read_manufacturer_string(
                                language,
                                &self.descriptor,
                                timeout,
                            ),
                            &none,
                        )
                    }
                    _ => none,
                };
                *cache = Some(manufacturer_string.clone());
                manufacturer_string
            }
        }
    }
    pub fn product_string(&self) -> String {
        let mut cache = self.cached_product_string.borrow_mut();
        match cache.clone() {
            Some(text) => text.clone(),
            None => {
                let timeout = Duration::from_secs(1);
                let none = "<none>".to_string();
                let product_string = match self.handle.read_languages(timeout) {
                    Ok(languages) => {
                        let language = languages[0];
                        clean_string(
                            self.handle
                                .read_product_string(language, &self.descriptor, timeout),
                            &none,
                        )
                    }
                    _ => none,
                };
                *cache = Some(product_string.clone());
                product_string
            }
        }
    }

    pub fn start(
        self,
        timeout: Duration,
    ) -> (
        tokio::sync::mpsc::Receiver<KeyEvent>,
        impl Fn() + Send + Sync + 'static,
        std::thread::JoinHandle<()>,
    ) {
        self.listening.store(true, Ordering::Relaxed);

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let stop_listening = Arc::clone(&self.listening);

        let handle = std::thread::spawn(move || {
            let mut buf = vec![0u8; self.max_packet_size];
            while self.listening.load(Ordering::Relaxed) {
                if let Some(ev) = self.read_key(&mut buf, timeout) {
                    if tx.blocking_send(ev).is_err() {
                        break;
                    }
                }
            }
        });

        let stop_fn = move || {
            stop_listening.store(false, Ordering::Relaxed);
        };

        (rx, stop_fn, handle)
    }

    pub fn read_key(&self, buf: &mut [u8], timeout: Duration) -> Option<KeyEvent> {
        match self.handle.read_interrupt(self.endpoint, buf, timeout) {
            Ok(n) => KeyParser::parse(&buf[..n]),
            Err(rusb::Error::Timeout) => None,
            Err(e) => {
                eprintln!("Error: {:?}", e);
                None
            }
        }
    }
}

fn clean_string(result: Result<String>, default: &String) -> String {
    result
        .map(|s| {
            s.chars()
                .filter(|c| !c.is_control() && ((*c as u32) < 0xFF))
                .collect::<String>()
                .trim()
                .to_string()
        })
        .unwrap_or(default.clone())
}

fn find_device(
    context: Context,
    vid: u16,
    pid: u16,
) -> rusb::Result<(rusb::Device<Context>, DeviceDescriptor)> {
    Ok(context
        .devices()?
        .iter()
        .map(|d| {
            let desc = d.device_descriptor().unwrap();
            (d, desc)
        })
        .find(|(_d, desc)| desc.vendor_id() == vid && desc.product_id() == pid)
        .ok_or(rusb::Error::NoDevice)?)
}
