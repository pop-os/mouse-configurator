use bitvec::prelude::*;
use std::{mem, num::NonZeroU8, str, sync::Arc};

use crate::{Button, Hid, HP_SIGNATURE};

fn u16_from_bytes(low: u8, high: u8) -> u16 {
    u16::from_le_bytes([low, high])
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
struct Header {
    signature: u16,
    #[allow(unused)]
    composit_device: u8,
    length: usize,
    sequence: u8,
}

impl Header {
    fn new(data: &[u8]) -> Option<Self> {
        Some(Self {
            signature: u16_from_bytes(*data.get(0)?, *data.get(1)? & 0b1111),
            composit_device: (data.get(1)? >> 4) & 0b1111,
            length: u16_from_bytes(*data.get(2)?, *data.get(3)? & 0b11) as usize,
            sequence: (*data.get(3)? >> 2) & 0b111111,
        })
    }

    fn kind(&self) -> Option<u16> {
        self.signature.checked_sub(HP_SIGNATURE)
    }
}

#[derive(Debug)]
pub enum Event {
    Firmware {
        version: (u16, u16, u16),
        device: String,
        serial: String,
    },
    Battery {
        low_level: u8,
        crit_level: u8,
        power_off_timeout: u8,
        auto_report_delay: u8,
        level: u8,
    },
    Buttons {
        total_buttons: u8,
        programmed_buttons: u8,
        host_id: u8,
        support_long_press: bool,
        support_double_press: bool,
        support_down_up_press: bool,
        support_simulate: bool,
        support_program_stop: bool,
        buttons: Vec<Button>,
    },
    Mouse {
        max_dpi: u16,
        min_dpi: u16,
        dpi: u16,
        step_dpi: u16,
        nb_sensitivity_wheel1: Option<NonZeroU8>,
        sensitivity_wheel1: u8,
        nb_sensitivity_wheel2: Option<NonZeroU8>,
        sensitivity_wheel2: u8,
        host_id: u8,
        cut_off_max: u8,
        cut_off: u8,
        support_left_handed: bool,
        left_handed: bool,
        support_no_save_to_flash: bool,
    },
}

pub struct HpMouseEvents {
    dev: Arc<Hid>,
    incoming: Vec<u8>,
    header: Header,
}

impl HpMouseEvents {
    pub(crate) fn new(dev: Arc<Hid>) -> Self {
        Self {
            dev,
            incoming: Vec::new(),
            header: Header::default(),
        }
    }

    fn report_1_packet_1(&mut self, data: &[u8]) -> Option<Event> {
        println!("Update {}", data.len());

        if data.len() <= 3 {
            // Buffer too small
            return None;
        }

        let firmware_version = u16_from_bytes(data[0], data[1]);
        let major_version = firmware_version / 1000;
        let minor_version = (firmware_version % 1000) / 10;
        let patch_version = firmware_version % 10;

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

        let device = str::from_utf8(items.get(0)?).ok()?;
        let serial = str::from_utf8(items.get(1)?).ok()?;

        Some(Event::Firmware {
            version: (major_version, minor_version, patch_version),
            device: device.to_string(),
            serial: serial.to_string(),
        })
    }

    fn report_1_packet_6(&mut self, data: &[u8]) -> Option<Event> {
        if data.len() <= 4 {
            // Buffer too small
            return None;
        }

        let low_level = data[0];
        let crit_level = data[1];
        let power_off_timeout = data[2];
        let auto_report_delay = data[3];
        let level = data[4];

        Some(Event::Battery {
            low_level,
            crit_level,
            power_off_timeout,
            auto_report_delay,
            level,
        })
    }

    fn report_1_packet_14(&mut self, data: &[u8]) -> Option<Event> {
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

        let flags = data[4].view_bits::<Lsb0>();
        let support_long_press = flags[0];
        let support_double_press = flags[1];
        let support_down_up_press = flags[2];
        let support_simulate = flags[3];
        let support_program_stop = flags[4];

        let mut buttons = Vec::with_capacity(programmed_buttons as usize);
        let mut i = 5;
        while buttons.len() < programmed_buttons as usize {
            if let Some((button, count)) = Button::decode(&data[i..]) {
                buttons.push(button);
                i += count;
            } else {
                break;
            }
        }

        for button in buttons.iter() {
            eprintln!("Action: {:?}", button.decode_action());
        }

        Some(Event::Buttons {
            total_buttons,
            programmed_buttons,
            host_id,
            support_long_press,
            support_double_press,
            support_down_up_press,
            support_simulate,
            support_program_stop,
            buttons,
        })
    }

    fn report_1_packet_18(&mut self, data: &[u8]) -> Option<Event> {
        if data.get(0) != Some(&0) {
            // Wrong command
            return None;
        }

        if data.len() <= 14 {
            // Buffer too small
            return None;
        }

        let max_dpi = u16_from_bytes(data[1], data[2]);
        let min_dpi = u16_from_bytes(data[3], data[4]);
        let dpi = u16_from_bytes(data[5], data[6]);
        let step_dpi = u16_from_bytes(data[7], data[8]);

        let nb_sensitivity_wheel1 = NonZeroU8::new(data[9] & 0b1111);
        let sensitivity_wheel1 = data[9] >> 4;
        let nb_sensitivity_wheel2 = NonZeroU8::new(data[10] & 0b1111);
        let sensitivity_wheel2 = data[10] >> 4;

        let host_id = data[11];
        let cut_off_max = data[12];
        let cut_off = data[13];

        let flags = data[14].view_bits::<Lsb0>();
        let support_left_handed = flags[0];
        let left_handed = flags[1];
        let support_no_save_to_flash = flags[2];

        Some(Event::Mouse {
            max_dpi,
            min_dpi,
            dpi,
            step_dpi,
            nb_sensitivity_wheel1,
            sensitivity_wheel1,
            nb_sensitivity_wheel2,
            sensitivity_wheel2,
            host_id,
            cut_off_max,
            cut_off,
            support_left_handed,
            left_handed,
            support_no_save_to_flash,
        })
    }

    fn report_1(&mut self, data: &[u8]) -> Result<Option<Event>, String> {
        let header = Header::new(data).ok_or_else(|| "Invalid header".to_string())?;

        let kind_opt = header.kind();
        println!(
            " signature {:04X} {:?} length {} sequence {}",
            header.signature, kind_opt, header.length, header.sequence
        );

        // Ensure signature is valid and can be converted to a packet kind
        let kind = kind_opt.ok_or_else(|| "Invalid header signature".to_string())?;

        // Insert new incoming packet if sequence is 0, verify there is no current one
        if header.sequence == 0 {
            if !self.incoming.is_empty() {
                return Err("Unexpected packet sequence 0".to_string());
            }
            self.header = header;
        // Get current incoming packet, verify that it exists
        } else {
            if self.incoming.is_empty() {
                return Err(format!("Unexpected packet sequence {}", header.sequence));
            }
            self.header.sequence += 1;
            if header != self.header {
                return Err(format!(
                    "Non-matching header. Expected: {:?} Found: {:?}",
                    self.header, header
                ));
            }
        }

        // Push back new data
        self.incoming.extend_from_slice(&data[4..]);

        // If we received enough data, truncate and return
        if self.incoming.len() >= header.length {
            let mut incoming = mem::take(&mut self.incoming);
            incoming.truncate(header.length);
            return Ok(match kind {
                1 => self.report_1_packet_1(&incoming),
                6 => self.report_1_packet_6(&incoming),
                14 => self.report_1_packet_14(&incoming),
                18 => self.report_1_packet_18(&incoming),
                _ => None,
            });
        }

        // No full packet yet
        Ok(None)
    }

    pub fn read(&mut self) -> Result<ReadRes, String> {
        let mut buf = [0; 4096];

        let len = self.dev.read(&mut buf).map_err(|x| x.to_string())?;
        eprintln!("HID read {}", len);

        if len == 0 {
            return Ok(ReadRes::EOF);
        }

        for i in 0..len {
            eprint!(" {:02x}", buf[i]);
        }
        eprintln!();

        match buf[0] {
            1 => {
                if let Some(packet) = self.report_1(&buf[1..len])? {
                    return Ok(ReadRes::Packet(packet));
                }
            }
            _ => {}
        }
        Ok(ReadRes::Continue)
    }
}

pub enum ReadRes {
    Packet(Event),
    Continue,
    EOF,
}

impl Iterator for HpMouseEvents {
    type Item = Result<Event, String>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.read() {
                Ok(ReadRes::Continue) => {
                    continue;
                }
                Ok(ReadRes::Packet(event)) => Some(Ok(event)),
                Ok(ReadRes::EOF) => None,
                Err(err) => Some(Err(err)),
            };
        }
    }
}
