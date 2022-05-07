use std::collections::HashMap;

use super::bindings::{HardwareButton, PresetBinding};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Binding {
    Preset(PresetBinding),
    // TODO Custom
}

impl Binding {
    pub fn label(&self) -> String {
        match self {
            Binding::Preset(binding) => binding.entry().label.to_string(),
        }
    }
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    pub bindings: HashMap<HardwareButton, Binding>,
    pub left_handed: bool,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct MouseInfo {
    // TODO: default?
    dpi: f64,
    serial: String,
}

struct Mouse {
    profiles: Vec<Profile>,
    info: MouseInfo,
}
