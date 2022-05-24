use std::collections::BTreeSet;

use mouse_configurator::{enumerate, Event, HpMouse};

fn hp_mouse(mouse: HpMouse) {
    println!("Found HP mouse");

    mouse.query_firmware().unwrap();
    mouse.query_battery().unwrap();
    mouse.query_button().unwrap();
    mouse.query_dpi().unwrap();

    for event in mouse.read() {
        println!("{:?}", event);
        if let Ok(Event::Buttons { buttons, .. }) = event {
            let host_ids: BTreeSet<_> = buttons.iter().map(|b| b.host_id).collect();
            println!("Buttons:");
            for host_id in host_ids {
                println!("    host_id: {}", host_id);
                let mut host_buttons: Vec<_> =
                    buttons.iter().filter(|b| b.host_id == host_id).collect();
                host_buttons.sort_by(|b1, b2| (b1.id, b1.press_type).cmp(&(b2.id, b2.press_type)));
                for button in host_buttons {
                    println!(
                        "        id: {}, press_type: {:?}, action: {:?}",
                        button.id,
                        button.press_type,
                        button.decode_action()
                    );
                }
            }
        }
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
