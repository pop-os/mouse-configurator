use hp_mouse_configurator::{enumerate, HpMouse};

fn hp_mouse(mut mouse: HpMouse) {
    println!("Found HP mouse");

    mouse.query_firmware().unwrap();
    mouse.query_battery().unwrap();
    mouse.query_button().unwrap();
    mouse.query_dpi().unwrap();

    for event in mouse.read() {
        println!("{:?}", event);
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
