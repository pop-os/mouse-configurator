use bitvec::prelude::*;
use std::fmt;

pub struct BitStream<'a> {
    bits: &'a BitSlice<u8, Lsb0>,
}

impl<'a> BitStream<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self::for_bitslice(data.view_bits())
    }

    fn for_bitslice(bits: &'a BitSlice<u8, Lsb0>) -> Self {
        Self { bits }
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
        assert!(
            count <= 8,
            "BitStream::bits: requested too many bits: {}",
            count
        );

        if let Some(bits) = self.bits.get(..count) {
            assert!(bits.len() == count);
            self.bits = &self.bits[count..];
            Some(bits.load_le::<u8>())
        } else {
            None
        }
    }

    #[allow(unused)]
    fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }

    #[allow(unused)]
    fn len(&self) -> usize {
        self.bits.len()
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum Value<T> {
    Var(u8),
    Const(T),
}

impl<T> From<T> for Value<T> {
    fn from(val: T) -> Self {
        Self::Const(val)
    }
}

impl<T: fmt::Debug> fmt::Debug for Value<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Var(var) => write!(f, "Var({:?})", var),
            Self::Const(val) => write!(f, "{:?}", val),
        }
    }
}

impl Default for Value<i16> {
    fn default() -> Self {
        Self::Const(0)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Op {
    Kill,
    Pause(Value<i16>),
    Mouse {
        auto_release: bool,
        buttons: Value<i16>,
        dx: Value<i16>,
        dy: Value<i16>,
        wheel1: Value<i16>,
        wheel2: Value<i16>,
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

impl Op {
    pub fn pause(value: impl Into<Value<i16>>) -> Self {
        Self::Pause(value.into())
    }

    pub fn mouse(
        auto_release: bool,
        buttons: impl Into<Value<i16>>,
        dx: impl Into<Value<i16>>,
        dy: impl Into<Value<i16>>,
        wheel1: impl Into<Value<i16>>,
        wheel2: impl Into<Value<i16>>,
    ) -> Self {
        Self::Mouse {
            auto_release,
            buttons: buttons.into(),
            dx: dx.into(),
            dy: dy.into(),
            wheel1: wheel1.into(),
            wheel2: wheel2.into(),
        }
    }

    pub fn key(auto_release: bool, payload: Vec<Value<i8>>) -> Self {
        Self::Key {
            auto_release,
            payload,
        }
    }

    pub fn media(auto_release: bool, payload: Vec<Value<i8>>) -> Self {
        Self::Media {
            auto_release,
            payload: payload,
        }
    }
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
fn get_value2(bitstream: &mut BitStream, _signed: bool) -> Result<Value<i16>, &'static str> {
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
    for i in payload {
        match i {
            Value::Var(var) => {
                push_bits(bitvec, 0b01, 2);
                push_bits(bitvec, *var, 4);
            }
            Value::Const(val) => {
                let bytes = val.to_le_bytes();
                if bytes[1] != 0 {
                    push_bits(bitvec, 0b11, 2);
                    push_bits(bitvec, bytes[0], 8);
                    push_bits(bitvec, bytes[1], 8);
                } else {
                    push_bits(bitvec, 0b10, 2);
                    push_bits(bitvec, bytes[0], 8);
                }
            }
        }
    }
    push_bits(bitvec, 0b00, 2);
}

fn push_value2(bitvec: &mut BitVec<u8, Lsb0>, value: &Value<i16>) {
    bitvec.push(matches!(value, Value::Const(_)));
    match value {
        Value::Var(var) => push_bits(bitvec, *var, 4),
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

pub fn encode_action(ops: &[Op]) -> Vec<u8> {
    let mut bitvec = BitVec::<u8, Lsb0>::new();
    for op in ops {
        match op {
            Op::Kill => {
                push_bits(&mut bitvec, 0, 5);
            }
            Op::Pause(value) => {
                push_bits(&mut bitvec, 21, 5);
                push_value2(&mut bitvec, value);
            }
            Op::Mouse {
                auto_release,
                buttons,
                dx,
                dy,
                wheel1,
                wheel2,
            } => {
                push_bits(&mut bitvec, 23, 5);
                bitvec.push(*auto_release);
                let mut payload: &[_] = &[*buttons, *dx, *dy, *wheel1, *wheel2];
                while payload.last() == Some(&Value::Const(0)) {
                    payload = &payload[..payload.len() - 1];
                }
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

pub fn decode_action(action: &[u8]) -> Result<Vec<Op>, String> {
    let mut bitstream = BitStream::new(action);

    let mut ops = Vec::new();
    while let Some(op) = bitstream.bits(5) {
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
                let payload = get_payload2(&mut bitstream)?;
                ops.push(Op::Mouse {
                    auto_release,
                    buttons: payload.get(0).copied().unwrap_or_default(),
                    dx: payload.get(1).copied().unwrap_or_default(),
                    dy: payload.get(2).copied().unwrap_or_default(),
                    wheel1: payload.get(3).copied().unwrap_or_default(),
                    wheel2: payload.get(4).copied().unwrap_or_default(),
                });
            }
            24 => {
                let auto_release = bitstream.bit().ok_or("Failed to read key auto release")?;
                let payload = get_payload(&mut bitstream)?;
                ops.push(Op::Key {
                    auto_release,
                    payload,
                });
            }
            27 => {
                let auto_release = bitstream.bit().ok_or("Failed to read key auto release")?;
                let payload = get_payload(&mut bitstream)?;
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

    if ops.len() > 1 && ops.last() == Some(&Op::Kill) {
        ops.pop();
    }

    Ok(ops)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum PressType {
    Normal = 0,
    Long = 1,
    Double = 2,
    Down = 3,
    Up = 4,
}

impl PressType {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Normal),
            1 => Some(Self::Long),
            2 => Some(Self::Double),
            3 => Some(Self::Down),
            4 => Some(Self::Up),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Button {
    pub id: u8,
    pub host_id: u8,
    pub press_type: PressType,
    pub(crate) action: Vec<u8>,
}

impl Button {
    pub fn new(id: u8, host_id: u8, press_type: PressType, action: &[Op]) -> Self {
        Self {
            id,
            host_id,
            press_type,
            action: encode_action(action),
        }
    }

    pub fn decode(data: &[u8]) -> Option<(Self, usize)> {
        if data.len() <= 3 {
            // Buffer too small
            return None;
        }

        let size = data[3] as usize;
        let button = Self {
            id: data[0],
            host_id: data[1],
            press_type: PressType::from_u8(data[2])?,
            action: data.get(4..4 + size)?.to_vec(),
        };
        Some((button, 4 + size))
    }

    pub fn encode(&self, data: &mut Vec<u8>) {
        data.push(self.id);
        data.push(self.host_id);
        data.push(self.press_type as u8);
        data.push(self.action.len() as u8);
        data.extend_from_slice(&self.action);
    }

    pub fn decode_action(&self) -> Result<Vec<Op>, String> {
        decode_action(&self.action)
    }
}

#[cfg(test)]
mod tests {
    use super::{Op::*, Value::*, *};

    fn zoom_in() -> Vec<Op> {
        vec![
            Op::key(false, vec![Const(1)]),
            Op::pause(100),
            Op::mouse(false, 0, 0, 0, 1, 0),
            Op::pause(100),
            Op::key(false, vec![]),
        ]
    }

    fn zoom_out() -> Vec<Op> {
        vec![
            Op::key(false, vec![Const(1)]),
            Op::pause(100),
            Op::mouse(false, 0, 0, 0, -1, 0),
            Op::pause(100),
            Op::key(false, vec![]),
        ]
    }

    #[test]
    fn test_value2() {
        let mut bitvec = BitVec::<u8, Lsb0>::new();
        push_value2(&mut bitvec, &Const(100));
        let mut bitstream = BitStream::for_bitslice(&bitvec);
        assert_eq!(get_value2(&mut bitstream, true).unwrap(), Const(100));
        assert_eq!(bitstream.len(), 0);
    }

    #[test]
    fn test_pause() {
        let pause = vec![Pause(Const(100))];
        assert_eq!(decode_action(&encode_action(&pause)).unwrap(), pause);
    }

    #[test]
    fn test_zoom_in() {
        let zoom_in = zoom_in();
        assert_eq!(decode_action(&encode_action(&zoom_in)).unwrap(), zoom_in);
    }

    #[test]
    fn test_zoom_out() {
        let zoom_out = zoom_out();
        assert_eq!(decode_action(&encode_action(&zoom_out)).unwrap(), zoom_out);
    }

    #[test]
    fn test_zoom_in_decode() {
        let zoom_in = zoom_in();
        let bytes = &[152, 1, 212, 200, 46, 1, 4, 16, 192, 0, 106, 100, 24];
        assert_eq!(decode_action(bytes).unwrap(), zoom_in);
    }

    #[test]
    fn test_zoom_out_decode() {
        let zoom_out = zoom_out();
        let bytes = &[152, 1, 212, 200, 46, 1, 4, 16, 192, 127, 106, 100, 24];
        assert_eq!(decode_action(bytes).unwrap(), zoom_out);
    }

    #[test]
    fn test_media_play_pause() {
        #[allow(overflowing_literals)]
        let play_pause = vec![Op::media(true, vec![Const(0xCD)])];
        assert_eq!(
            decode_action(&encode_action(&play_pause)).unwrap(),
            play_pause
        );
    }
}
