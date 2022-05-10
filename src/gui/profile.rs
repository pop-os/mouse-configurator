use std::{collections::HashMap, env, path::PathBuf};

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

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MouseInfo {
    // TODO: default?
    pub dpi: Option<f64>,
    pub serial: String,
}

struct Mouse {
    profiles: Vec<Profile>,
    info: MouseInfo,
}

fn data_dir() -> PathBuf {
    if let Ok(dir) = env::var("XDG_DATA_HOME") {
        dir.into()
    } else if let Ok(dir) = env::var("HOME") {
        let mut path = PathBuf::from(dir);
        path.push(".local/share");
        path
    } else {
        panic!("`XDG_DATA_HOME` and `HOME` undefined")
    }
}

fn app_data_dir() -> PathBuf {
    let mut dir = data_dir();
    dir.push("hp-mouse-configurator");
    if let Err(err) = std::fs::create_dir_all(&dir) {
        panic!("Failed to create directory `{}`: {}", dir.display(), err);
    }
    dir
}
