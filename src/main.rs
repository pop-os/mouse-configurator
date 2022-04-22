use std::fs::File;

use hp_mouse_configurator::HpMouse;

const HP_VENDOR_ID: u16 = 0x03F0;
const BT_PRODUCT_ID: u16 = 0x524A;
const USB_PRODUCT_ID: u16 = 0x544A;

fn hp_mouse(mut mouse: HpMouse) {
    println!("Found HP mouse");

    // Send query for normal info
    mouse.write_report_1(0, &[]).unwrap();

    // Send query for battery info
    {
        let low_level = 0xFF; // do not set
        let crit_level = 0xFF; // do not set
        let power_off_timeout = 0xFF; // do not set
        let auto_report_delay = 0x06; // 60 seconds
        mouse
            .write_report_1(
                5,
                &[low_level, crit_level, power_off_timeout, auto_report_delay],
            )
            .unwrap();
    }

    // Send query for button info
    {
        let command = 0; // request status command
        let host_id = 0; // current host
        mouse.write_report_1(13, &[command, host_id]).unwrap();
    }

    // Send query for DPI info
    {
        let host_id = 0; // current host
        let command = 4; // request status command, no save to flash not set
        mouse
            .write_report_1(
                17,
                &[
                    host_id, command, 0, 0, // payload
                ],
            )
            .unwrap();
    }

    loop {
        println!("{:?}", mouse.read().unwrap());
    }
}

fn parse_hid_id(id: &str) -> Option<(u16, u16)> {
    let mut iter = id.split(':');
    let _ = iter.next()?;
    let vendor_id = u16::from_str_radix(iter.next()?, 16).ok()?;
    let product_id = u16::from_str_radix(iter.next()?, 16).ok()?;
    Some((vendor_id, product_id))
}

fn main() {
    let mut enumerator = udev::Enumerator::new().unwrap();
    enumerator.match_subsystem("hidraw").unwrap();
    for device in enumerator.scan_devices().unwrap() {
        let hid_device = match device.parent_with_subsystem("hid").unwrap() {
            Some(dev) => dev,
            None => { continue; }
        };
        let (vendor_id, product_id) = match hid_device.property_value("HID_ID").and_then(|x| parse_hid_id(x.to_str()?)) {
            Some(id) => id,
            None => { continue; }
        };
        let devnode = match device.devnode() {
            Some(devnode) => devnode,
            None => { continue; }
        };
        println!(
            "ID {:04x}:{:04x}",
            vendor_id,
            product_id,
        );
        match (vendor_id, product_id) {
            //TODO: also support HP mouse via bluetooth
            (HP_VENDOR_ID, USB_PRODUCT_ID | BT_PRODUCT_ID) => match File::options().read(true).write(true).open(devnode) {
                Ok(ok) => hp_mouse(HpMouse::new(ok)),
                Err(err) => {
                    eprintln!("failed to open HP mouse: {}", err);
                }
            },
            _ => (),
        }
    }
    /*
        Err(err) => {
            eprintln!("failed to list HID devices: {}", err);
        }
    }
    */
}
