use std::{
    collections::BTreeMap,
    str,
};

const HP_VENDOR_ID: u16 = 0x03F0;
const BT_PRODUCT_ID: u16 = 0x524A;
const USB_PRODUCT_ID: u16 = 0x544A;

const HP_SIGNATURE: u16 = 0xCF3;

fn u16_from_bytes(low: u8, high: u8) -> u16 {
    (low as u16) | ((high as u16) << 8)
}

pub struct HpMouse {
    dev: hidapi::HidDevice,
    incoming: BTreeMap<u16, Vec<u8>>,
}

impl HpMouse {
    pub fn new(dev: hidapi::HidDevice) -> Self {
        Self {
            dev,
            incoming: BTreeMap::new(),
        }
    }

    fn report_1_packet_1(&mut self, data: &[u8]) -> Option<()> {
        println!(" update {}", data.len());

        if data.len() <= 3 {
            // Buffer too small
            return None;
        }

        let firmware_version = u16_from_bytes(data[0], data[1]);
        let major_version = firmware_version / 1000;
        let minor_version = (firmware_version % 1000) / 10;
        let patch_version = firmware_version % 10;
        println!(
            "  firmware version {}.{}.{}",
            major_version,
            minor_version,
            patch_version
        );

        let mut items = Vec::with_capacity(2);
        let mut i = 4;
        while i < data.len() {
            let size = data[i] as usize;
            i += 1;

            let mut item = Vec::with_capacity(size);
            while i < data.len() && item.len() < size {
                item.push(data[i]);
                i += 1;
            }
            items.push(item);
        }

        println!("  device: {:?}", items.get(0).map(|x| str::from_utf8(x)));
        println!("  serial: {:?}", items.get(1).map(|x| str::from_utf8(x)));

        None
    }

    fn report_1_packet_6(&mut self, data: &[u8]) -> Option<()> {
        println!(" update battery {}", data.len());

        if data.len() <= 4 {
            // Buffer too small
            return None;
        }

        let low_level = data[0];
        let crit_level = data[1];
        let power_off_timeout = data[2];
        let auto_report_delay = data[3];
        let level = data[4];

        println!("  low level: {}", low_level);
        println!("  critical level: {}", crit_level);
        println!("  power off timeout: {}", power_off_timeout);
        println!("  auto report delay: {}", auto_report_delay);
        println!("  level: {}", level);

        None
    }

    fn report_1_packet_14(&mut self, data: &[u8]) -> Option<()> {
        println!(" update buttons {}", data.len());

        if data.get(0) != Some(&0) {
            // Wrong command
            return None;
        }

        if data.len() <= 4 {
            // Buffer too small
            return None;
        }

        let total_buttons = data[1];
        let programmed_buttons = data[2];
        let host_id = data[3];
        let flags = data[4];

        println!("  total buttons: {}", total_buttons);
        println!("  programmed_buttons: {}", programmed_buttons);
        println!("  host id: {}", host_id);
        println!("  flags: {:#x}", flags);

        #[derive(Debug)]
        struct Button {
            id: u8,
            host_id: u8,
            press_type: u8,
            action: Vec<u8>,
        }

        let mut buttons = Vec::with_capacity(programmed_buttons as usize);
        let mut i = 5;
        while buttons.len() < programmed_buttons as usize {
            if data.len() <= i + 3 {
                // Buffer too small
                break;
            }

            let size = data[i + 3] as usize;
            let mut button = Button {
                id: data[i + 0],
                host_id: data[i + 1],
                press_type: data[i + 2],
                action: Vec::with_capacity(size),
            };
            i += 4;

            while i < data.len() && button.action.len() < size {
                button.action.push(data[i]);
                i += 1;
            }
            buttons.push(button);
        }

        for button in buttons.iter() {
            println!("{:#x?}", button);
        }

        None
    }

    fn report_1_packet_18(&mut self, data: &[u8]) -> Option<()> {
        println!(" update mouse {}", data.len());

        if data.get(0) != Some(&0) {
            // Wrong command
            return None;
        }

        if data.len() <= 8 {
            // Buffer too small
            return None;
        }

        let max_dpi = u16_from_bytes(data[1], data[2]);
        let min_dpi = u16_from_bytes(data[3], data[4]);
        let dpi = u16_from_bytes(data[5], data[6]);
        let step_dpi = u16_from_bytes(data[7], data[8]);
        //TODO: more settings

        println!("  max dpi: {}", max_dpi);
        println!("  min dpi: {}", min_dpi);
        println!("  dpi: {}", dpi);
        println!("  step dpi: {}", step_dpi);

        None
    }

    fn report_1(&mut self, data: &[u8]) -> Option<()> {
        if data.len() <= 3 {
            // Buffer too small
            return None;
        }

        let signature = u16_from_bytes(data[0], data[1] & 0b1111);
        let composit_device = (data[1] >> 4) & 0b1111;
        let length = u16_from_bytes(data[2], data[3] & 0b11) as usize;
        let sequence = (data[3] >> 2) & 0b111111;

        let kind_opt = signature.checked_sub(HP_SIGNATURE);
        println!(
            " signature {:04X} {:?} length {} sequence {}",
            signature,
            kind_opt,
            length,
            sequence
        );

        // Ensure signature is valid and can be converted to a packet kind
        let kind = kind_opt?;

        //TODO: replace asserts with errors

        // Insert new incoming packet if sequence is 0, assert that there is no current one
        if sequence == 0 {
            assert_eq!(
                self.incoming.insert(kind, Vec::with_capacity(length)),
                None
            );
        }

        // Get current incoming packet, assert that it exists
        let mut incoming = self.incoming.remove(&kind).unwrap();

        // Assert that incoming packet capacity is equal to requested length
        assert_eq!(incoming.capacity(), length);

        // Push back new data
        incoming.extend_from_slice(&data[4..]);

        // If we received enough data, truncate and return
        if incoming.len() >= length {
            incoming.truncate(length);
            return match kind {
                1 => self.report_1_packet_1(&incoming),
                6 => self.report_1_packet_6(&incoming),
                14 => self.report_1_packet_14(&incoming),
                18 => self.report_1_packet_18(&incoming),
                _ => None,
            };
        }

        // Re-add incoming packet, ensuring no other packet is overwritten
        assert_eq!(self.incoming.insert(kind, incoming), None);

        // No full packet yet
        None
    }

    pub fn read(&mut self) -> hidapi::HidResult<Option<()>> {
        let mut buf = [0; 4096];
        let len = self.dev.read(&mut buf)?;
        eprintln!("HID read {}", len);

        if len == 0 {
            return Ok(None);
        }

        for i in 0..len {
            eprint!(" {:02x}", buf[i]);
        }
        eprintln!();

        for i in 0..len {
            let c = match buf[i] {
                b @ 0x20..=0x7E => b as char,
                _ => ' ',
            };
            eprint!("  {}", c);
        }
        eprintln!();

        Ok(match buf[0] {
            1 => self.report_1(&buf[1..len]),
            _ => None,
        })
    }
}

fn hp_mouse(mut mouse: HpMouse) {
    println!("Found HP mouse");

    //TODO: cleanup queries

    // Send query for normal info
    {
        let report = 1;
        let signature = HP_SIGNATURE + 0;
        let length = 0;

        let mut data = [0; 21];
        data[0] = report;
        data[1] = signature as u8;
        data[2] = (signature >> 8) as u8;
        data[3] = length;
        // data[4] is sequence, length high
        mouse.dev.write(&data).unwrap();
    }

    // Send query for battery info
    {
        let report = 1;
        let signature = HP_SIGNATURE + 5;
        let length = 4;
        let low_level = 0xFF; // do not set
        let crit_level = 0xFF; // do not set
        let power_off_timeout = 0xFF; // do not set
        let auto_report_delay = 0x06; // 60 seconds

        let mut data = [0; 21];
        data[0] = report;
        data[1] = signature as u8;
        data[2] = (signature >> 8) as u8;
        data[3] = length;
        // data[4] is sequence, length high
        data[5] = low_level;
        data[6] = crit_level;
        data[7] = power_off_timeout;
        data[8] = auto_report_delay;
        mouse.dev.write(&data).unwrap();
    }

    // Send query for button info
    {
        let report = 1;
        let signature = HP_SIGNATURE + 13;
        let length = 2;
        let command = 0; // request status command
        let host_id = 0; // current host

        let mut data = [0; 21];
        data[0] = report;
        data[1] = signature as u8;
        data[2] = (signature >> 8) as u8;
        data[3] = length;
        // data[4] is sequence, length high
        data[5] = command;
        data[6] = host_id;
        mouse.dev.write(&data).unwrap();
    }

    // Send query for DPI info
    {
        let report = 1;
        let signature = HP_SIGNATURE + 17;
        let length = 4;
        let host_id = 0; // current host
        let command = 4; // request status command, no save to flash not set

        let mut data = [0; 21];
        data[0] = report;
        data[1] = signature as u8;
        data[2] = (signature >> 8) as u8;
        data[3] = length;
        // data[4] is sequence, length high
        data[5] = host_id;
        data[6] = command;
        // data[7] and data[8] are payload
        mouse.dev.write(&data).unwrap();
    }

    loop {
        mouse.read().unwrap();
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
                        }
                        _ => (),
                    },
                    (HP_VENDOR_ID, USB_PRODUCT_ID) => match (info.usage_page(), info.usage()) {
                        (0xFF00, 0x0001) => match info.open_device(&api) {
                            Ok(ok) => hp_mouse(HpMouse::new(ok)),
                            Err(err) => {
                                eprintln!("failed to open HP mouse: {}", err);
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }
        },
        Err(err) => {
            eprintln!("failed to list HID devices: {}", err);
        },
    }
}
