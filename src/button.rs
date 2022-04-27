use bitvec::prelude::*;

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
pub enum Value<T> {
    Var(u8),
    Const(T),
}

#[derive(Debug)]
pub enum Op {
    Kill,
    Pause(Value<i16>),
    Mouse {
        auto_release: bool,
        payload: Vec<Value<i16>>,
    },
    Key {
        auto_release: bool,
        payload: Vec<Value<i8>>,
    },
    Media {
        auto_release: bool,
        payload: Vec<Value<i8>>,
    },
}

fn get_payload(bitstream: &mut BitStream) -> Result<Vec<Value<i8>>, &'static str> {
    let mut values = Vec::new();
    loop {
        match bitstream.bits(2).ok_or("Failed to read payload kind")? {
            0b00 => {
                break;
            }
            0b01 => {
                let nibble = bitstream.bits(4).ok_or("Failed to read payload nibble")?;
                values.push(Value::Var(nibble));
            }

            0b10 => {
                let byte = bitstream.bits(8).ok_or("Failed to read payload byte")?;
                values.push(Value::Const(byte as i8));
            }
            0b11 => {
                let byte1 = bitstream.bits(8).ok_or("Failed to read payload byte")?;
                let byte2 = bitstream.bits(8).ok_or("Failed to read payload byte")?;
                values.push(Value::Const(byte1 as i8));
                values.push(Value::Const(byte2 as i8));
            }
            _ => unreachable!(),
        }
    }
    Ok(values)
}

fn get_payload2(bitstream: &mut BitStream) -> Result<Vec<Value<i16>>, &'static str> {
    let mut values = Vec::new();
    loop {
        match bitstream.bits(2).ok_or("Failed to read payload kind")? {
            0b00 => {
                break;
            }
            0b01 => {
                let nibble = bitstream.bits(4).ok_or("Failed to read payload nibble")?;
                values.push(Value::Var(nibble));
            }

            0b10 => {
                let byte = bitstream.bits(8).ok_or("Failed to read payload byte")?;
                values.push(Value::Const(i16::from(byte as i8)));
            }
            0b11 => {
                let byte1 = bitstream.bits(8).ok_or("Failed to read payload byte")?;
                let byte2 = bitstream.bits(8).ok_or("Failed to read payload byte")?;
                values.push(Value::Const(i16::from_le_bytes([byte1, byte2])));
            }
            _ => unreachable!(),
        }
    }
    Ok(values)
}

// XXX signed
fn get_value2(bitstream: &mut BitStream, signed: bool) -> Result<Value<i16>, &'static str> {
    if !bitstream.bit().ok_or("Failed to read value bit")? {
        Ok(Value::Var(
            bitstream.bits(4).ok_or("Failed to read value nibble")?,
        ))
    } else if !bitstream.bit().ok_or("Failed to read value bit")? {
        let byte = bitstream.bits(8).ok_or("Failed to read value byte")?;
        Ok(Value::Const(i16::from(byte as i8)))
    } else {
        Ok(Value::Const(i16::from_le_bytes([
            bitstream.bits(8).ok_or("Failed to read value byte")?,
            bitstream.bits(8).ok_or("Failed to read value byte")?,
        ])))
    }
}

fn push_bits(bitvec: &mut BitVec<u8, Lsb0>, byte: u8, count: usize) {
    bitvec.extend_from_bitslice(&[byte].view_bits::<Lsb0>()[..count]);
}

fn push_payload(bitvec: &mut BitVec<u8, Lsb0>, payload: &[Value<i8>]) {
    let mut prev_const: Option<u8> = None;
    for i in payload {
        match i {
            Value::Var(var) => {
                if let Some(prev_val) = prev_const.take() {
                    push_bits(bitvec, 0b10, 2);
                    push_bits(bitvec, prev_val, 8);
                }
                push_bits(bitvec, 0b01, 2);
                push_bits(bitvec, *var, 4);
            }
            Value::Const(val) => {
                if let Some(prev_val) = prev_const.take() {
                    push_bits(bitvec, 0b11, 2);
                    push_bits(bitvec, prev_val, 8);
                    push_bits(bitvec, *val as u8, 8);
                } else {
                    prev_const = Some(*val as u8);
                }
            }
        }
    }
    if let Some(prev_val) = prev_const.take() {
        push_bits(bitvec, 0b10, 2);
        push_bits(bitvec, prev_val, 8);
    }
    push_bits(bitvec, 0b00, 2);
}

fn push_payload2(bitvec: &mut BitVec<u8, Lsb0>, payload: &[Value<i16>]) {
    let mut prev_const: Option<u8> = None;
    for i in payload {
        match i {
            Value::Var(var) => {
                if let Some(prev_val) = prev_const.take() {
                    push_bits(bitvec, 0b10, 2);
                    push_bits(bitvec, prev_val, 8);
                }
                push_bits(bitvec, 0b01, 2);
                push_bits(bitvec, *var, 4);
            }
            Value::Const(val) => {
                let bytes = val.to_le_bytes();
                if bytes[1] != 0 {
                    if let Some(prev_val) = prev_const.take() {
                        push_bits(bitvec, 0b10, 2);
                        push_bits(bitvec, prev_val, 8);
                    }
                    push_bits(bitvec, 0b11, 2);
                    push_bits(bitvec, bytes[0], 8);
                    push_bits(bitvec, bytes[1], 8);
                } else {
                    if let Some(prev_val) = prev_const.take() {
                        push_bits(bitvec, 0b11, 2);
                        push_bits(bitvec, prev_val, 8);
                        push_bits(bitvec, bytes[0], 8);
                    } else {
                        prev_const = Some(bytes[0]);
                    }
                }
            }
        }
    }
    if let Some(prev_val) = prev_const.take() {
        push_bits(bitvec, 0b10, 2);
        push_bits(bitvec, prev_val as u8, 8);
    }
    push_bits(bitvec, 0b00, 2);
}

fn push_value2(bitvec: &mut BitVec<u8, Lsb0>, value: &Value<i16>) {
    match value {
        Value::Var(var) => {
            bitvec.push(false);
            push_bits(bitvec, 0, 1);
            push_bits(bitvec, *var, 4);
        }
        Value::Const(val) => {
            let bytes = val.to_le_bytes();
            bitvec.push(bytes[1] != 0);
            push_bits(bitvec, bytes[0], 8);
            if bytes[1] != 0 {
                push_bits(bitvec, bytes[1], 8);
            }
        }
    }
}

fn encode_action(ops: &[Op]) -> Vec<u8> {
    let mut bitvec = BitVec::<u8, Lsb0>::new();
    for op in ops {
        match op {
            Op::Kill => {
                push_bits(&mut bitvec, 0, 5);
            }
            Op::Pause(value) => {
                push_value2(&mut bitvec, value);
            }
            Op::Mouse {
                auto_release,
                payload,
            } => {
                push_bits(&mut bitvec, 23, 5);
                bitvec.push(*auto_release);
                push_payload2(&mut bitvec, payload);
            }
            Op::Key {
                auto_release,
                payload,
            } => {
                push_bits(&mut bitvec, 24, 5);
                bitvec.push(*auto_release);
                push_payload(&mut bitvec, payload);
            }
            Op::Media {
                auto_release,
                payload,
            } => {
                push_bits(&mut bitvec, 27, 5);
                bitvec.push(*auto_release);
                push_payload(&mut bitvec, payload);
            }
        }
    }
    bitvec.into()
}

#[derive(Debug)]
pub struct Button {
    pub(crate) id: u8,
    pub(crate) host_id: u8,
    pub(crate) press_type: u8,
    pub(crate) action: Vec<u8>,
}

impl Button {
    pub fn decode_action(&self) -> Result<Vec<Op>, String> {
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
                    ops.push(Op::Pause(get_value2(&mut bitstream, false)?));
                }
                23 => {
                    let auto_release = bitstream.bit().ok_or("Failed to read key auto release")?;
                    let mut payload = get_payload2(&mut bitstream)?;
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
