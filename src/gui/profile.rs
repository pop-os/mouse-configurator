use relm4::{send, Sender};
use std::{collections::HashMap, env, path::PathBuf};

use super::{
    bindings::{HardwareButton, PresetBinding},
    worker::{DeviceId, WorkerMsg},
};
use hp_mouse_configurator::Button;

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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

struct MouseConfig {
    profiles: Vec<Profile>,
    profile_num: usize,
    info: MouseInfo,
}

impl MouseConfig {
    fn profile(&self) -> Option<&Profile> {
        self.profiles.get(self.profile_num)
    }

    fn profile_mut(&mut self) -> Option<&mut Profile> {
        self.profiles.get_mut(self.profile_num)
    }
}

#[derive(Default)]
struct MouseState {
    connected: bool,
    battery_percent: Option<u8>,
    dpi: Option<f64>,
    profile: Option<Profile>,
}
// TODO: way to convert state on device to profile? Include option for unrecognized binding?
// left_handed is sent seperately from bindings?

impl MouseState {
    fn disconnect(&mut self) {
        *self = Self::default();
    }
}

// Update bindings in state to match config, and generate messages to apply changes
fn apply_profile_diff(
    device_id: DeviceId,
    config: &MouseConfig,
    state: &mut MouseState,
    sender: Sender<WorkerMsg>,
) {
    match (config.profile(), &mut state.profile) {
        (Some(config_profile), Some(state_profile)) => {
            for i in HardwareButton::iter() {
                let config_binding = config_profile.bindings.get(&i);
                let state_binding = state_profile.bindings.get(&i);
                if state_binding != config_binding {
                    if let Some(binding) = config_binding {
                        state_profile.bindings.insert(i, binding.clone());
                    } else {
                        state_profile.bindings.remove(&i);
                    }
                    let binding = match config_binding {
                        Some(Binding::Preset(preset)) => &preset.entry().binding,
                        None => &[] as &[_],
                    };
                    let button = Button::new(i as u8, 1, 0, binding); // XXX
                    send!(sender, WorkerMsg::SetBinding(device_id.clone(), button));
                }
            }

            if state_profile.left_handed != config_profile.left_handed {
                state_profile.left_handed = config_profile.left_handed;
                send!(
                    sender,
                    WorkerMsg::SetLeftHanded(device_id, config_profile.left_handed)
                );
            }
        }
        _ => {}
    }
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
