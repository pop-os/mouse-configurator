#![allow(overflowing_literals)]

use once_cell::sync::Lazy;
use std::collections::HashMap;

use hp_mouse_configurator::{Op, Value::*};

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum HardwareButton {
    Right = 0,
    Middle = 1,
    LeftBottom = 2,
    LeftTop = 3,
    ScrollLeft = 4,
    ScrollRight = 5,
    LeftCenter = 6,
}

impl HardwareButton {
    pub fn def_binding(self) -> &'static Entry {
        match self {
            Self::Right => Entry::for_binding(&[Op::mouse(true, 2, 0, 0, 0, 0)]),
            Self::Middle => Entry::for_binding(&[Op::mouse(true, 4, 0, 0, 0, 0)]),
            Self::LeftBottom => Entry::for_binding(&[Op::mouse(true, 8, 0, 0, 0, 0)]),
            Self::LeftTop => Entry::for_binding(&[Op::mouse(true, 16, 0, 0, 0, 0)]),
            Self::ScrollLeft => Entry::for_binding(&[Op::mouse(false, 0, 0, 0, 0, -1)]),
            Self::ScrollRight => Entry::for_binding(&[Op::mouse(false, 0, 0, 0, 0, 1)]),
            Self::LeftCenter => Entry::for_binding(&[Op::key(true, vec![Const(0), Const(0x2B)])]),
        }
        .unwrap()
    }
}

pub struct Category {
    pub label: &'static str,
    pub entries: Vec<Entry>,
}

pub struct Entry {
    pub label: &'static str,
    pub binding: Vec<Op>,
}

pub static BINDINGS: Lazy<Vec<Category>> = Lazy::new(|| {
    vec![
        Category {
            label: "Mouse Controls",
            entries: vec![
                Entry {
                    label: "Right Click",
                    binding: vec![Op::mouse(true, 2, 0, 0, 0, 0)],
                },
                Entry {
                    label: "Left Click",
                    binding: vec![Op::mouse(true, 1, 0, 0, 0, 0)],
                },
                Entry {
                    label: "Middle Click",
                    binding: vec![Op::mouse(true, 4, 0, 0, 0, 0)],
                },
                Entry {
                    label: "Scroll Left",
                    binding: vec![Op::mouse(false, 0, 0, 0, 0, -1)],
                },
                Entry {
                    label: "Scroll Right",
                    binding: vec![Op::mouse(false, 0, 0, 0, 0, 1)],
                },
                Entry {
                    label: "Back",
                    binding: vec![Op::mouse(true, 8, 0, 0, 0, 0)],
                },
                Entry {
                    label: "Forward",
                    binding: vec![Op::mouse(true, 16, 0, 0, 0, 0)],
                },
                Entry {
                    // XXX
                    label: "Switch App",
                    binding: vec![Op::key(true, vec![Const(0), Const(0x2B)])], // super + tab
                },
            ],
        },
        Category {
            label: "Media Controls",
            entries: vec![
                Entry {
                    label: "Volume Down",
                    binding: vec![Op::media(true, vec![Const(0xEA)])],
                },
                Entry {
                    label: "Volume Up",
                    binding: vec![Op::media(true, vec![Const(0xE9)])],
                },
                Entry {
                    label: "Next Track",
                    binding: vec![Op::media(true, vec![Const(0xB5)])],
                },
                Entry {
                    label: "Previous Track",
                    binding: vec![Op::media(true, vec![Const(0xB6)])],
                },
                Entry {
                    label: "Play / Pause",
                    binding: vec![Op::media(true, vec![Const(0xCD)])],
                },
                Entry {
                    label: "Mute",
                    binding: vec![Op::media(true, vec![Const(0xE2)])],
                },
            ],
        },
    ]
});

impl Entry {
    fn for_binding(binding: &[Op]) -> Option<&'static Entry> {
        static ENTRY_FOR_BINDING: Lazy<HashMap<&[Op], &Entry>> = Lazy::new(|| {
            let mut map = HashMap::new();
            for category in &*BINDINGS {
                for entry in &category.entries {
                    map.insert(entry.binding.as_slice(), entry);
                }
            }
            map
        });
        ENTRY_FOR_BINDING.get(binding).copied()
    }
}

#[cfg(test)]
mod tests {
    use hp_mouse_configurator::button::{decode_action, encode_action};

    use super::*;

    #[test]
    fn invertible_bindings() {
        for category in &*BINDINGS {
            for entry in &category.entries {
                assert_eq!(
                    decode_action(&encode_action(&entry.binding)).unwrap(),
                    entry.binding
                );
            }
        }
    }
}
