#![allow(overflowing_literals)]

use once_cell::sync::Lazy;

use hp_mouse_configurator::{Op, Value::*};

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
