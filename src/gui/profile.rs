use std::collections::HashMap;

use super::bindings::{HardwareButton, PresetBinding};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum Binding {
    Preset(PresetBinding),
    // TODO Custom
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct Profile {
    // XXX serializable; binding name or custom binding
    bindings: HashMap<HardwareButton, Binding>,
    left_handed: bool,
}
