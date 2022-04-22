use bitvec::prelude::*;
use std::{fs::File, io::{self, Read, Write}, mem, str};

const HP_SIGNATURE: u16 = 0xCF3;

fn u16_from_bytes(low: u8, high: u8) -> u16 {
    u16::from_le_bytes([low, high])
}

pub struct BitStream<'a> {
    data: &'a [u8],
    i: usize,
}

impl<'a> BitStream<'a> {
    fn bit(&mut self) -> Option<bool> {
        let bits = self.data.view_bits::<Lsb0>();
        if self.i < bits.len() {
            let value = bits[self.i];
            self.i += 1;
            Some(value)
        } else {
            None
        }
    }

    fn bits(&mut self, count: usize) -> Option<u8> {
        if count > 8 {
            println!("BitStream::bits: requested too many bits: {}", count);
            return None;
        }

        let bits = self.data.view_bits::<Lsb0>();
        let end = self.i + count;
        if end <= bits.len() {
            let value = bits[self.i..end].load_le::<u8>();
            self.i = end;
            Some(value)
        } else {
            None
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Header {
    signature: u16,
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
pub struct Button {
    id: u8,
    host_id: u8,
    press_type: u8,
    action: Vec<u8>,
}

impl Button {
    fn decode_action(&self) {
        let mut bitstream = BitStream {
            data: &self.action,
            i: 0,
        };

        loop {
            let op = match bitstream.bits(5) {
                Some(some) => some,
                None => {
                    println!(" Failed to read OP");
                    break;
                }
            };

            println!("OP {}", op);
            match op {
                0 => {
                    println!(" Finish");
                    break;
                }
                24 => {
                    println!(" Keyboard");

                    let auto_release = match bitstream.bit() {
                        Some(some) => some,
                        None => {
                            println!("  Failed to read key auto release");
                            break;
                        }
                    };
                    println!("  Auto release: {}", auto_release);

                    let mut payload = Vec::new();
                    loop {
                        let bytes = match bitstream.bits(2) {
                            Some(kind) => match kind {
                                0 => break,
                                //TODO: 1, using "math variables"
                                2 => 1,
                                3 => 2,
                                _ => {
                                    println!("  Unsupported payload kind {}", kind);
                                    break;
                                }
                            },
                            None => {
                                println!("  Failed to read payload kind");
                                break;
                            }
                        };

                        for byte in 0..bytes {
                            match bitstream.bits(8) {
                                Some(byte) => payload.push(byte),
                                None => {
                                    println!("  Failed to read payload byte");
                                    break;
                                }
                            }
                        }
                    }
                    println!("  Payload: {:#x?}", payload);
                }
                27 => {
                    println!(" Media");

                    let auto_release = match bitstream.bit() {
                        Some(some) => some,
                        None => {
                            println!("  Failed to read key auto release");
                            break;
                        }
                    };
                    println!("  Auto release: {}", auto_release);

                    let mut payload = Vec::new();
                    loop {
                        let bytes = match bitstream.bits(2) {
                            Some(kind) => match kind {
                                0 => break,
                                //TODO: 1, using "math variables"
                                2 => 1,
                                3 => 2,
                                _ => {
                                    println!("  Unsupported payload kind {}", kind);
                                    break;
                                }
                            },
                            None => {
                                println!("  Failed to read payload kind");
                                break;
                            }
                        };

                        for byte in 0..bytes {
                            match bitstream.bits(8) {
                                Some(byte) => payload.push(byte),
                                None => {
                                    println!("  Failed to read payload byte");
                                    break;
                                }
                            }
                        }
                    }
                    println!("  Payload: {:#x?}", payload);
                }
                _ => {
                    println!(" Unsupported OP");
                    break;
                }
            }
        }
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
        flags: u8, // XXX Hex?
        buttons: Vec<Button>,
    },
    Mouse {
        max_dpi: u16,
        min_dpi: u16,
        dpi: u16,
        step_dpi: u16,
    },
}

pub struct HpMouse {
    pub dev: File,
    incoming: Vec<u8>,
    header: Header,
}

impl HpMouse {
    pub fn new(dev: File) -> Self {
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
        let flags = data[4];

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
            button.decode_action();
        }

        Some(Event::Buttons {
            total_buttons,
            programmed_buttons,
            host_id,
            flags,
            buttons,
        })
    }

    fn report_1_packet_18(&mut self, data: &[u8]) -> Option<Event> {
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

        Some(Event::Mouse {
            max_dpi,
            min_dpi,
            dpi,
            step_dpi,
        })
    }

    fn report_1(&mut self, data: &[u8]) -> Option<Event> {
        let header = Header::new(data)?;

        let kind_opt = header.kind();
        println!(
            " signature {:04X} {:?} length {} sequence {}",
            header.signature, kind_opt, header.length, header.sequence
        );

        // Ensure signature is valid and can be converted to a packet kind
        let kind = kind_opt?;

        //TODO: replace asserts with errors

        // Insert new incoming packet if sequence is 0, assert that there is no current one
        if header.sequence == 0 {
            assert_eq!(self.incoming.len(), 0);
            self.header = header;
        // Get current incoming packet, assert that it exists
        } else {
            assert_eq!(header.signature, self.header.signature);
            assert_eq!(header.length, self.header.length);
            assert_eq!(header.sequence, self.header.sequence + 1);
            self.header.sequence += 1;
        }

        // Push back new data
        self.incoming.extend_from_slice(&data[4..]);

        // If we received enough data, truncate and return
        if self.incoming.len() >= header.length {
            let mut incoming = mem::take(&mut self.incoming);
            incoming.truncate(header.length);
            return match kind {
                1 => self.report_1_packet_1(&incoming),
                6 => self.report_1_packet_6(&incoming),
                14 => self.report_1_packet_14(&incoming),
                18 => self.report_1_packet_18(&incoming),
                _ => None,
            };
        }

        // No full packet yet
        None
    }

    //TODO: support multi report packets
    pub fn write_report_1(&mut self, kind: u16, packet: &[u8]) -> io::Result<()> {
        let report = 1;
        let signature = HP_SIGNATURE + kind;
        assert_eq!(signature & 0xF000, 0);

        let mut data = [0; 21];
        data[0] = report;
        data[1] = signature as u8;
        data[2] = (signature >> 8) as u8;
        data[3] = packet.len() as u8;
        // data[4] is sequence, length high
        for i in 0..packet.len() {
            data[5 + i] = packet[i];
        }

        let len = self.dev.write(&data)?;
        eprintln!("HID write {}", len);

        for i in 0..len {
            eprint!(" {:02x}", data[i]);
        }
        eprintln!();

        Ok(())
    }

    pub fn read(&mut self) -> io::Result<Option<Event>> {
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

        Ok(match buf[0] {
            1 => self.report_1(&buf[1..len]),
            _ => None,
        })
    }
}
