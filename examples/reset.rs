fn main() {
    for device in hp_mouse_configurator::enumerate().unwrap() {
        let device = device.open().unwrap();
        for id in 0..7 {
            let button = hp_mouse_configurator::Button::new(id, 1, 0, &[]);
            device.set_button(button, false).unwrap();
        }
    }
}
