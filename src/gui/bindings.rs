// TODO custom bindings
// - Need way to get label, binding, from json representation

#![allow(overflowing_literals)]

use once_cell::sync::Lazy;
use std::collections::HashMap;

use hp_mouse_configurator::{Op, Value::*};

// TODO better naming? Important if serialized in json.
#[repr(u8)]
#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
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
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..7).map(|i| Self::from_u8(i).unwrap())
    }

    pub fn from_u8(num: u8) -> Option<Self> {
        match num {
            0 => Some(Self::Right),
            1 => Some(Self::Middle),
            2 => Some(Self::LeftBottom),
            3 => Some(Self::LeftTop),
            4 => Some(Self::ScrollLeft),
            5 => Some(Self::ScrollRight),
            6 => Some(Self::LeftCenter),
            _ => None,
        }
    }

    pub fn def_binding(self) -> &'static Entry {
        match self {
            Self::Right => Entry::for_binding(&[Op::mouse(true, 2, 0, 0, 0, 0)]),
            Self::Middle => Entry::for_binding(&[Op::mouse(true, 4, 0, 0, 0, 0)]),
            Self::LeftBottom => Entry::for_binding(&[Op::mouse(true, 8, 0, 0, 0, 0)]),
            Self::LeftTop => Entry::for_binding(&[Op::mouse(true, 16, 0, 0, 0, 0)]),
            Self::ScrollLeft => Entry::for_binding(&[Op::mouse(false, 0, 0, 0, 0, -1)]), // XXX not same as default?
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

#[derive(Debug)]
pub struct Entry {
    pub id: PresetBinding,
    pub label: &'static str,
    pub binding: Vec<Op>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PresetBinding {
    RightClick,
    LeftClick,
    MiddleClick,
    ScrollLeft,
    ScrollRight,
    Back,
    Forward,
    SwitchApp,
    VolumeDown,
    VolumeUp,
    NextTrack,
    PreviousTrack,
    PlayPause,
    Mute,
}

impl PresetBinding {
    pub fn entry(self) -> &'static Entry {
        static ENTRY_FOR_PRESET: Lazy<HashMap<PresetBinding, &Entry>> = Lazy::new(|| {
            let mut map = HashMap::new();
            for category in &*BINDINGS {
                for entry in &category.entries {
                    map.insert(entry.id, entry);
                }
            }
            map
        });
        ENTRY_FOR_PRESET.get(&self).unwrap().clone()
    }
}

pub static BINDINGS: Lazy<Vec<Category>> = Lazy::new(|| {
    use PresetBinding::*;
    vec![
        Category {
            label: "Mouse Controls",
            entries: vec![
                Entry {
                    id: RightClick,
                    label: "Right Click",
                    binding: vec![Op::mouse(true, 2, 0, 0, 0, 0)],
                },
                Entry {
                    id: LeftClick,
                    label: "Left Click",
                    binding: vec![Op::mouse(true, 1, 0, 0, 0, 0)],
                },
                Entry {
                    id: MiddleClick,
                    label: "Middle Click",
                    binding: vec![Op::mouse(true, 4, 0, 0, 0, 0)],
                },
                Entry {
                    id: ScrollLeft,
                    label: "Scroll Left",
                    binding: vec![Op::mouse(false, 0, 0, 0, 0, -1)],
                },
                Entry {
                    id: ScrollRight,
                    label: "Scroll Right",
                    binding: vec![Op::mouse(false, 0, 0, 0, 0, 1)],
                },
                Entry {
                    id: Back,
                    label: "Back",
                    binding: vec![Op::mouse(true, 8, 0, 0, 0, 0)],
                },
                Entry {
                    id: Forward,
                    label: "Forward",
                    binding: vec![Op::mouse(true, 16, 0, 0, 0, 0)],
                },
                Entry {
                    // XXX
                    id: SwitchApp,
                    label: "Switch App",
                    binding: vec![Op::key(true, vec![Const(0), Const(0x2B)])], // super + tab
                },
            ],
        },
        Category {
            label: "Media Controls",
            entries: vec![
                Entry {
                    id: VolumeDown,
                    label: "Volume Down",
                    binding: vec![Op::media(true, vec![Const(0xEA)])],
                },
                Entry {
                    id: VolumeUp,
                    label: "Volume Up",
                    binding: vec![Op::media(true, vec![Const(0xE9)])],
                },
                Entry {
                    id: NextTrack,
                    label: "Next Track",
                    binding: vec![Op::media(true, vec![Const(0xB5)])],
                },
                Entry {
                    id: PreviousTrack,
                    label: "Previous Track",
                    binding: vec![Op::media(true, vec![Const(0xB6)])],
                },
                Entry {
                    id: PlayPause,
                    label: "Play / Pause",
                    binding: vec![Op::media(true, vec![Const(0xCD)])],
                },
                Entry {
                    id: Mute,
                    label: "Mute",
                    binding: vec![Op::media(true, vec![Const(0xE2)])],
                },
            ],
        },
    ]
});

impl Entry {
    pub fn for_binding(binding: &[Op]) -> Option<&'static Entry> {
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
