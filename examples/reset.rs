fn main() {
    for device in mouse_configurator::enumerate().unwrap() {
        device.open().unwrap().reset().unwrap();
    }
}
