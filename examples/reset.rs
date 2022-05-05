fn main() {
    for device in hp_mouse_configurator::enumerate().unwrap() {
        device.open().unwrap().reset().unwrap();
    }
}
