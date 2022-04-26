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
    bits: &'a BitSlice<u8, Lsb0>,
}

impl<'a> BitStream<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            bits: data.view_bits(),
        }
    }

    fn bit(&mut self) -> Option<bool> {
        if let Some(value) = self.bits.get(0) {
            self.bits = &self.bits[1..];
            Some(*value)
        } else {
            None
        }
    }

    fn bits(&mut self, count: usize) -> Option<u8> {
        if count > 8 {
            println!("BitStream::bits: requested too many bits: {}", count);
            return None;
        }

        if let Some(bits) = self.bits.get(..count) {
            self.bits = &self.bits[count..];
            Some(bits.load_le::<u8>())
        } else {
            None
        }
    }

    fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }
}

#[derive(Debug)]
pub struct Button {
    id: u8,
    host_id: u8,
    press_type: u8,
    action: Vec<u8>,
}

#[derive(Debug)]
enum Value {
    Var(u8),
    Const(u16),
}

#[derive(Debug)]
enum Op {
    Kill,
    Pause(Value),
    Mouse {
        auto_release: bool,
        payload: Vec<Value>,
    },
    Key {
        auto_release: bool,
        payload: Vec<Value>,
    },
    Media {
        auto_release: bool,
        payload: Vec<Value>,
    },
}

// TODO: specific errors
fn get_payload(bitstream: &mut BitStream) -> Result<Vec<Value>, &'static str> {
    let mut values = Vec::new();
    loop {
        values.push(
            match bitstream.bits(2).ok_or("Failed to read payload kind")? {
                0b00 => {
                    break;
                }
                0b01 => Value::Var(bitstream.bits(4).ok_or("Failed to read payload nibble")?),
                0b10 => Value::Const(
                    bitstream
                        .bits(8)
                        .ok_or("Failed to read payload byte")?
                        .into(),
                ),
                0b11 => Value::Const(u16::from_le_bytes([
                    bitstream.bits(8).ok_or("Failed to read payload byte")?,
                    bitstream.bits(8).ok_or("Failed to read payload byte")?,
                ])),
                _ => unreachable!(),
            },
        );
    }
    Ok(values)
}

fn get_value2(bitstream: &mut BitStream) -> Result<Value, &'static str> {
    if !bitstream.bit().ok_or("Failed to read value bit")? {
        Ok(Value::Var(
            bitstream.bits(4).ok_or("Failed to read value nibble")?,
        ))
    } else if !bitstream.bit().ok_or("Failed to read value bit")? {
        Ok(Value::Const(
            bitstream.bits(8).ok_or("Failed to read value byte")?.into(),
        ))
    } else {
        // XXX signed vs unsigned? Cast to i32.
        Ok(Value::Const(u16::from_le_bytes([
            bitstream.bits(8).ok_or("Failed to read value byte")?,
            bitstream.bits(8).ok_or("Failed to read value byte")?,
        ])))
    }
}

fn push_bits(bitvec: &mut BitVec<u8, Lsb0>, byte: u8, count: usize) {
    bitvec.extend_from_bitslice(&[byte].view_bits::<Lsb0>()[..count]);
}

fn encode_action(ops: &[Op]) -> Vec<u8> {
    let mut bitvec = BitVec::<u8, Lsb0>::new();
    for op in ops {
        match op {
            Op::Kill => {
                push_bits(&mut bitvec, 0, 5);
            }
            Op::Pause(value) => {}
            Op::Mouse {
                auto_release,
                payload,
            } => {}
            Op::Key {
                auto_release,
                payload,
            } => {
                push_bits(&mut bitvec, 24, 5);
                bitvec.push(*auto_release);
                // XXX push payload
            }
            Op::Media {
                auto_release,
                payload,
            } => {}
        }
    }
    bitvec.into()
}

impl Button {
    fn decode_action(&self) -> Result<Vec<Op>, String> {
        let mut bitstream = BitStream::new(&self.action);

        let mut ops = Vec::new();
        while !bitstream.is_empty() {
            let op = bitstream.bits(5).ok_or("Failed to read OP")?;
            match op {
                0 => {
                    ops.push(Op::Kill);
                    break;
                }
                21 => {
                    ops.push(Op::Pause(get_value2(&mut bitstream)?));
                }
                23 => {
                    let auto_release = bitstream.bit().ok_or("Failed to read key auto release")?;
                    let mut payload = get_payload(&mut bitstream)?;
                    ops.push(Op::Mouse {
                        auto_release,
                        payload,
                    });
                }
                24 => {
                    let auto_release = bitstream.bit().ok_or("Failed to read key auto release")?;
                    let mut payload = get_payload(&mut bitstream)?;
                    ops.push(Op::Key {
                        auto_release,
                        payload,
                    });
                }
                27 => {
                    let auto_release = bitstream.bit().ok_or("Failed to read key auto release")?;
                    let mut payload = get_payload(&mut bitstream)?;
                    ops.push(Op::Media {
                        auto_release,
                        payload,
                    });
                }
                _ => {
                    return Err(format!("Unsupported OP {}", op));
                }
            }
        }

        Ok(ops)
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
