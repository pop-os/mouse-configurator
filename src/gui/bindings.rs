// TODO custom bindings
// - Need way to get label, binding, from json representation

use once_cell::sync::Lazy;
use serde::de::{self, Error};
use std::collections::HashMap;

use crate::keycode::*;
use mouse_configurator::{Op, Value::*};

// TODO better naming? Important if serialized in json.
#[repr(u8)]
#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub enum HardwareButton {
    Right = 0,
    Middle = 1,
    LeftBottom = 2,
    LeftTop = 3,
    ScrollLeft = 4,
    ScrollRight = 5,
    LeftCenter = 6,
}

// Serialize as int to be future-proof for more devices, and not depend on naming
impl serde::Serialize for HardwareButton {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (*self as u8).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for HardwareButton {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let num = u8::deserialize(deserializer)?;
        Self::from_u8(num).ok_or_else(|| {
            D::Error::invalid_value(de::Unexpected::Unsigned(num.into()), &"button index 0-6")
        })
    }
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
            Self::Right => PresetBinding::RightClick,
            Self::Middle => PresetBinding::MiddleClick,
            Self::LeftBottom => PresetBinding::Back,
            Self::LeftTop => PresetBinding::Forward,
            Self::ScrollLeft => PresetBinding::ScrollLeft, // XXX not same as default?
            Self::ScrollRight => PresetBinding::ScrollRight,
            Self::LeftCenter => PresetBinding::SwitchApplications,
        }
        .entry()
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
    pub keybind: Option<&'static str>,
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
    SwitchApplications,
    Disabled,
    VolumeDown,
    VolumeUp,
    NextTrack,
    PreviousTrack,
    PlayPause,
    Mute,
    Copy,
    Cut,
    Paste,
    Undo,
    SelectAll,
    Redo,
    Find,
    Refresh,
    OpenTab,
    CloseTab,
    NewWindow,
    Home,
    SaveBookmark,
    ZoomIn,
    ZoomOut,
    MoveToWorkspaceBelow,
    MoveToWorkspaceAbove,
    WorkspacesOverview,
    ToggleFullscreen,
    Restart,
    LogOut,
    LockScreen,
    Super,
    PrintScreen,
    ShowApplications,
    ShowLauncher,
    BrightnessUp,
    BrightnessDown,
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
                    keybind: None,
                },
                Entry {
                    id: LeftClick,
                    label: "Left Click",
                    binding: vec![Op::mouse(true, 1, 0, 0, 0, 0)],
                    keybind: None,
                },
                Entry {
                    id: MiddleClick,
                    label: "Middle Click",
                    binding: vec![Op::mouse(true, 4, 0, 0, 0, 0)],
                    keybind: None,
                },
                Entry {
                    id: ScrollLeft,
                    label: "Scroll Left",
                    binding: vec![Op::mouse(false, 0, 0, 0, 0, -1)],
                    keybind: None,
                },
                Entry {
                    id: ScrollRight,
                    label: "Scroll Right",
                    binding: vec![Op::mouse(false, 0, 0, 0, 0, 1)],
                    keybind: None,
                },
                Entry {
                    id: Back,
                    label: "Back",
                    binding: vec![Op::mouse(true, 8, 0, 0, 0, 0)],
                    keybind: None,
                },
                Entry {
                    id: Forward,
                    label: "Forward",
                    binding: vec![Op::mouse(true, 16, 0, 0, 0, 0)],
                    keybind: None,
                },
                Entry {
                    id: Disabled,
                    label: "Disabled",
                    binding: vec![Op::Kill],
                    keybind: None,
                },
            ],
        },
        Category {
            label: "Media Controls",
            entries: vec![
                Entry {
                    id: VolumeDown,
                    label: "Volume Down",
                    binding: vec![Op::media(true, vec![Const(MEDIA_VolumeDown)])],
                    keybind: None,
                },
                Entry {
                    id: VolumeUp,
                    label: "Volume Up",
                    binding: vec![Op::media(true, vec![Const(MEDIA_VolumeUp)])],
                    keybind: None,
                },
                Entry {
                    id: NextTrack,
                    label: "Next Track",
                    binding: vec![Op::media(true, vec![Const(MEDIA_NextSong)])],
                    keybind: None,
                },
                Entry {
                    id: PreviousTrack,
                    label: "Previous Track",
                    binding: vec![Op::media(true, vec![Const(MEDIA_PreviousSong)])],
                    keybind: None,
                },
                Entry {
                    id: PlayPause,
                    label: "Play / Pause",
                    binding: vec![Op::media(true, vec![Const(MEDIA_PlayPause)])],
                    keybind: None,
                },
                Entry {
                    id: Mute,
                    label: "Mute",
                    binding: vec![Op::media(true, vec![Const(MEDIA_Mute)])],
                    keybind: None,
                },
            ],
        },
        Category {
            label: "Edit Features",
            entries: vec![
                Entry {
                    id: Copy,
                    label: "Copy",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_C)])],
                    keybind: Some("Ctrl+C"),
                },
                Entry {
                    id: Cut,
                    label: "Cut",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_X)])],
                    keybind: Some("Ctrl+X"),
                },
                Entry {
                    id: Paste,
                    label: "Paste",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_V)])],
                    keybind: Some("Ctrl+V"),
                },
                Entry {
                    id: Undo,
                    label: "Undo",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_Z)])],
                    keybind: Some("Ctrl+Z"),
                },
                Entry {
                    id: Redo,
                    label: "Redo",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_Y)])],
                    keybind: Some("Ctrl+Y"),
                },
                Entry {
                    id: SelectAll,
                    label: "Select All",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_A)])],
                    keybind: Some("Ctrl+A"),
                },
                Entry {
                    id: Find,
                    label: "Find",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_F)])],
                    keybind: Some("Ctrl+F"),
                },
            ],
        },
        Category {
            label: "App/Browser Controls",
            entries: vec![
                Entry {
                    id: Refresh,
                    label: "Refresh",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_R)])],
                    keybind: Some("Ctrl+R"),
                },
                Entry {
                    id: OpenTab,
                    label: "Open New Tab/Page",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_T)])],
                    keybind: Some("Ctrl+T"),
                },
                Entry {
                    id: CloseTab,
                    label: "Close Current Tab/Page",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_W)])],
                    keybind: Some("Ctrl+W"),
                },
                Entry {
                    id: NewWindow,
                    label: "New Window",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_N)])],
                    keybind: Some("Ctrl+N"),
                },
                Entry {
                    id: Home,
                    label: "Home",
                    binding: vec![Op::key(true, vec![Const(MOD_Alt), Const(KEY_Home)])],
                    keybind: Some("Alt+Home"),
                },
                Entry {
                    id: SaveBookmark,
                    label: "Save Page as Bookmark",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_D)])],
                    keybind: Some("Ctrl+D"),
                },
                Entry {
                    id: ZoomIn,
                    label: "Zoom In",
                    binding: vec![Op::key(
                        true,
                        vec![Const(MOD_Ctrl | MOD_Shift), Const(KEY_Equal)],
                    )],
                    keybind: Some("Ctrl +"),
                },
                Entry {
                    id: ZoomIn,
                    label: "Zoom Out",
                    binding: vec![Op::key(true, vec![Const(MOD_Ctrl), Const(KEY_Minus)])],
                    keybind: Some("Ctrl -"),
                },
            ],
        },
        Category {
            label: "System and Window Controls",
            entries: vec![
                Entry {
                    id: MoveToWorkspaceBelow,
                    label: "Move To Workspace Below",
                    binding: vec![Op::key(
                        true,
                        vec![Const(MOD_Ctrl | MOD_Super), Const(KEY_Down)],
                    )],
                    keybind: Some("Ctrl+Super+Down"),
                },
                Entry {
                    id: MoveToWorkspaceAbove,
                    label: "Move To Workspace Above",
                    binding: vec![Op::key(
                        true,
                        vec![Const(MOD_Ctrl | MOD_Super), Const(KEY_Up)],
                    )],
                    keybind: Some("Ctrl+Super+Up"),
                },
                Entry {
                    id: WorkspacesOverview,
                    label: "Workspaces Overview",
                    binding: vec![Op::key(true, vec![Const(MOD_Super), Const(KEY_D)])],
                    keybind: Some("Ctrl+D"),
                },
                Entry {
                    id: ToggleFullscreen,
                    label: "Toggle Fullscreen",
                    binding: vec![Op::key(true, vec![Const(0), Const(KEY_F11)])],
                    keybind: Some("F11"),
                },
                Entry {
                    id: Restart,
                    label: "Restart",
                    binding: vec![Op::media(true, vec![Const(MEDIA_Restart)])],
                    keybind: None,
                },
                Entry {
                    id: LogOut,
                    label: "Log Out",
                    binding: vec![Op::key(
                        true,
                        vec![Const(MOD_Ctrl | MOD_Alt), Const(KEY_Delete)],
                    )],
                    keybind: Some("Ctrl+Alt+Delete"),
                },
                Entry {
                    id: LockScreen,
                    label: "Lock Screen",
                    binding: vec![Op::key(true, vec![Const(MOD_Super), Const(KEY_Esc)])],
                    keybind: Some("Super+Escape"),
                },
                Entry {
                    id: Super,
                    label: "Super",
                    binding: vec![Op::key(true, vec![Const(0), Const(KEY_LeftSuper)])],
                    keybind: None,
                },
                Entry {
                    id: PrintScreen,
                    label: "Print Screen",
                    binding: vec![Op::key(true, vec![Const(0), Const(KEY_SysRq)])],
                    keybind: None,
                },
                Entry {
                    id: ShowApplications,
                    label: "Show All Applications",
                    binding: vec![Op::key(true, vec![Const(MOD_Super), Const(KEY_A)])],
                    keybind: Some("Super+A"),
                },
                Entry {
                    id: SwitchApplications,
                    label: "Switch Applications",
                    binding: vec![Op::key(true, vec![Const(MOD_Super), Const(KEY_Tab)])],
                    keybind: Some("Super+Tab"),
                },
                Entry {
                    id: ShowLauncher,
                    label: "Open Launcher",
                    binding: vec![Op::key(true, vec![Const(MOD_Super), Const(KEY_Slash)])],
                    keybind: Some("Super+/"),
                },
                Entry {
                    id: BrightnessUp,
                    label: "Increase Display Brightness",
                    binding: vec![Op::media(true, vec![Const(MEDIA_BrightnessUp)])],
                    keybind: None,
                },
                Entry {
                    id: BrightnessDown,
                    label: "Decrease Display Brightness",
                    binding: vec![Op::media(true, vec![Const(MEDIA_BrightnessDown)])],
                    keybind: None,
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
    use mouse_configurator::button::{decode_action, encode_action};

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
