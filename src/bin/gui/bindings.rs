use hp_mouse_configurator::Op;

pub struct Category {
    pub label: &'static str,
    pub entries: &'static [Entry],
}

pub struct Entry {
    pub label: &'static str,
    pub binding: &'static [Op],
}

pub static BINDINGS: &[Category] = &[
    Category {
        label: "mouse-controls",
        entries: &[
            Entry {
                label: "right-click",
                binding: &[],
            },
            Entry {
                label: "left-click",
                binding: &[],
            },
            Entry {
                label: "middle-click",
                binding: &[],
            },
        ],
    },
    Category {
        label: "media-controls",
        entries: &[
            Entry {
                label: "volume-down",
                binding: &[],
            },
            Entry {
                label: "volume-up",
                binding: &[],
            },
        ],
    },
];
