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

fn main() {
    match hidapi::HidApi::new() {
        Ok(api) => {
            for info in api.device_list() {
                println!(
                    "ID {:04x}:{:04x} IF {} USAGE {:04x}:{:04x}",
                    info.vendor_id(),
                    info.product_id(),
                    info.interface_number(),
                    info.usage_page(),
                    info.usage()
                );
                match (info.vendor_id(), info.product_id()) {
                    //TODO: also support HP mouse via bluetooth
                    (HP_VENDOR_ID, BT_PRODUCT_ID) => match (info.usage_page(), info.usage()) {
                        (0xFF00, 0x0001) => match info.open_device(&api) {
                            Ok(ok) => hp_mouse(HpMouse::new(ok)),
                            Err(err) => {
                                eprintln!("failed to open HP mouse: {}", err);
                            }
                        },
                        _ => (),
                    },
                    (HP_VENDOR_ID, USB_PRODUCT_ID) => match (info.usage_page(), info.usage()) {
                        (0xFF00, 0x0001) => match info.open_device(&api) {
                            Ok(ok) => hp_mouse(HpMouse::new(ok)),
                            Err(err) => {
                                eprintln!("failed to open HP mouse: {}", err);
                            }
                        },
                        _ => (),
                    },
                    _ => (),
                }
            }
        }
        Err(err) => {
            eprintln!("failed to list HID devices: {}", err);
        }
    }
}
