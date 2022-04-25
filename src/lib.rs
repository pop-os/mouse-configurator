use bitvec::prelude::*;
use std::{io, path::Path, sync::Arc};

mod enumerate;
pub use enumerate::enumerate;
mod event;
pub use event::{Event, HpMouseEventIterator};
mod hid;
use hid::Hid;

const HP_SIGNATURE: u16 = 0xCF3;

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

pub struct HpMouse {
    dev: Arc<Hid>,
}

impl HpMouse {
    pub fn open_devnode(path: &Path) -> io::Result<Self> {
        Ok(Self {
            dev: Arc::new(Hid::open(path)?),
        })
    }

    //TODO: support multi report packets
    pub fn write_report_1(&self, kind: u16, packet: &[u8]) -> io::Result<()> {
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

    /// Send query for firmware info
    pub fn query_firmware(&self) -> io::Result<()> {
        self.write_report_1(0, &[])
    }

    /// Send query for battery info
    pub fn query_battery(&self) -> io::Result<()> {
        let low_level = 0xFF; // do not set
        let crit_level = 0xFF; // do not set
        let power_off_timeout = 0xFF; // do not set
        let auto_report_delay = 0x06; // 60 seconds
        self.write_report_1(
            5,
            &[low_level, crit_level, power_off_timeout, auto_report_delay],
        )
    }

    /// Send query for button info
    pub fn query_button(&self) -> io::Result<()> {
        let command = 0; // request status command
        let host_id = 0; // current host
        self.write_report_1(13, &[command, host_id])
    }

    /// Send query for DPI info
    pub fn query_dpi(&self) -> io::Result<()> {
        let host_id = 0; // current host
        let command = 4; // request status command, no save to flash not set
        self.write_report_1(
            17,
            &[
                host_id, command, 0, 0, // payload
            ],
        )
    }

    pub fn set_dpi(&self, dpi: u16) -> io::Result<()> {
        let host_id = 0; // current host
        let command = 0; // set dpi
        let dpi = dpi.to_le_bytes();
        self.write_report_1(17, &[host_id, command, dpi[0], dpi[1]])
    }

    // Using multiple readers will result in inconsistent behavior
    pub fn read(&self) -> HpMouseEventIterator {
        HpMouseEventIterator::new(self.dev.clone())
    }
}
