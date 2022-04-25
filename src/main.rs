use std::fs::File;

use hp_mouse_configurator::{enumerate, HpMouse};

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
    match enumerate() {
        Ok(devices) => {
            for device in devices {
                println!("{:?}", device);
                match device.open() {
                    Ok(mouse) => hp_mouse(mouse),
                    Err(err) => eprintln!("failed to open HP mouse: {}", err),
                }
            }
        }
        Err(err) => eprintln!("failed to list HID devices: {}", err),
    }
}
