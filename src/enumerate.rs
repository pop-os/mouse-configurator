use nix::poll::{poll, PollFd, PollFlags};
use std::{io, os::unix::io::AsRawFd, path::PathBuf};

use super::HpMouse;

const HP_VENDOR_ID: u16 = 0x03F0;
const BT_PRODUCT_ID: u16 = 0x524A;
const USB_PRODUCT_ID: u16 = 0x544A;

#[derive(Debug)]
pub struct DeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub interface: Option<u8>,
    pub devnode: PathBuf,
}

impl DeviceInfo {
    pub fn open(&self) -> io::Result<HpMouse> {
        HpMouse::open_devnode(&self.devnode)
    }
}

fn parse_hid_id(id: &str) -> Option<(u16, u16)> {
    let mut iter = id.split(':');
    let _ = iter.next()?;
    let vendor_id = u16::from_str_radix(iter.next()?, 16).ok()?;
    let product_id = u16::from_str_radix(iter.next()?, 16).ok()?;
    Some((vendor_id, product_id))
}

fn get_interface_number(device: &udev::Device) -> Option<u8> {
    let interface = device
        .parent_with_subsystem_devtype("usb", "usb_interface")
        .ok()??;
    interface
        .attribute_value("bInterfaceNumber")?
        .to_str()?
        .parse()
        .ok()
}

fn match_device(device: &udev::Device) -> Option<DeviceInfo> {
    let hid_device = device.parent_with_subsystem("hid").ok()??;
    let (vendor_id, product_id) = hid_device
        .property_value("HID_ID")
        .and_then(|x| parse_hid_id(x.to_str()?))?;
    let interface = get_interface_number(&device);
    let devnode = device.devnode()?;
    match (vendor_id, product_id, interface) {
        (HP_VENDOR_ID, USB_PRODUCT_ID, Some(1)) | (HP_VENDOR_ID, BT_PRODUCT_ID, _) => {
            Some(DeviceInfo {
                vendor_id,
                product_id,
                interface,
                devnode: devnode.to_owned(),
            })
        }
        _ => None,
    }
}

pub fn enumerate() -> io::Result<Vec<DeviceInfo>> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("hidraw")?;
    Ok(enumerator
        .scan_devices()?
        .into_iter()
        .filter_map(|device| match_device(&device))
        .collect())
}

struct PollMonitorIter {
    monitor: udev::MonitorSocket,
    poll: bool,
}

impl Iterator for PollMonitorIter {
    type Item = DeviceInfo;

    fn next(&mut self) -> Option<DeviceInfo> {
        loop {
            if self.poll {
                let fd = PollFd::new(self.monitor.as_raw_fd(), PollFlags::POLLIN);
                let _ = poll(&mut [fd], -1);
            }
            self.poll = false;
            if let Some(evt) = self.monitor.next() {
                if evt.event_type() == udev::EventType::Add {
                    if let Some(device_info) = match_device(&evt.device()) {
                        return Some(device_info);
                    }
                }
            } else {
            }
            self.poll = true;
        }
    }
}

pub fn monitor() -> io::Result<impl Iterator<Item = DeviceInfo> + 'static> {
    let monitor = udev::MonitorBuilder::new()?
        .match_subsystem("hidraw")?
        .listen()?;
    Ok(PollMonitorIter {
        monitor,
        poll: true,
    })
}
