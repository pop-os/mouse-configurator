use relm4::{send, RelmWorker};
use std::{
    collections::HashMap,
    env,
    fs::File,
    path::{Path, PathBuf},
};

use super::{
    bindings::{Entry, HardwareButton, PresetBinding},
    worker::{DeviceId, WorkerModel, WorkerMsg},
    AppModel,
};
use hp_mouse_configurator::{Button, PressType};

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Binding {
    Preset(PresetBinding),
    // TODO Custom
    // Binding read from device, that isn't recognized
    Unknown,
}

impl Binding {
    pub fn label(&self) -> String {
        match self {
            Binding::Preset(binding) => binding.entry().label.to_string(),
            Binding::Unknown => "Unknown".to_string(),
        }
    }
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    pub name: Option<String>,
    pub bindings: HashMap<HardwareButton, Binding>,
    pub left_handed: bool,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MouseConfig {
    // Must always be non-empty
    profiles: Vec<Profile>,
    // Must Always be in range
    profile_num: usize,
    pub dpi: f64,
    pub device: String,
}

impl MouseConfig {
    // XXX Default DPI should depend on device model
    pub fn new(device: String) -> Self {
        let profiles = (0..4).map(|_| Profile::default()).collect();
        Self {
            profiles,
            profile_num: 0,
            device,
            dpi: 1200.,
        }
    }

    pub fn profile(&self) -> &Profile {
        &self.profiles[self.profile_num]
    }

    pub fn profile_mut(&mut self) -> &mut Profile {
        &mut self.profiles[self.profile_num]
    }

    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }

    pub fn profiles_mut(&mut self) -> &mut [Profile] {
        &mut self.profiles
    }

    pub fn select_profile(&mut self, profile: usize) {
        if profile < self.profiles.len() {
            self.profile_num = profile;
        }
    }

    pub fn profile_num(&self) -> usize {
        self.profile_num
    }

    pub fn import(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|x| x.to_string())?;
        serde_json::from_reader(file).map_err(|x| x.to_string())
    }

    pub fn export(&self, path: &Path) -> Result<(), String> {
        let file = File::create(path).map_err(|x| x.to_string())?;
        serde_json::to_writer(file, self).map_err(|x| x.to_string())
    }
}

#[derive(Default)]
pub struct MouseState {
    pub connected: bool,
    pub battery_percent: Option<u8>,
    pub dpi: Option<f64>,
    pub bindings: Option<HashMap<HardwareButton, Binding>>,
    pub left_handed: Option<bool>,
    pub firmware_version: Option<(u16, u16, u16)>,
}

impl MouseState {
    pub fn set_bindings_from_buttons(&mut self, host_id: u8, buttons: &[Button]) {
        let mut bindings = HashMap::new();

        for button in buttons {
            if button.host_id != host_id {
                continue;
            }
            let id = match HardwareButton::from_u8(button.id) {
                Some(id) => id,
                None => {
                    eprintln!("Unrecognized button id: {}", button.id);
                    continue;
                }
            };
            let binding = match button.decode_action() {
                Ok(action) => {
                    if let Some(entry) = Entry::for_binding(&action) {
                        Binding::Preset(entry.id)
                    } else {
                        eprintln!("Unrecognized action: {:?}", action);
                        Binding::Unknown
                    }
                }
                Err(err) => {
                    eprintln!("Unable to decode button action: {}", err);
                    Binding::Unknown
                }
            };
            bindings.insert(id, binding);
        }

        self.bindings = Some(bindings);
    }
}

impl MouseState {
    pub fn set_disconnected(&mut self) {
        *self = Self::default();
    }

    pub fn set_connected(&mut self) {
        *self = Self::default();
        self.connected = true;
    }
}

// Update bindings in state to match config, and generate messages to apply changes
pub(super) fn apply_profile_diff(
    device_id: DeviceId,
    config: &MouseConfig,
    state: &mut MouseState,
    worker: &RelmWorker<WorkerModel, AppModel>,
) {
    let config_profile = config.profile();

    if let Some(state_bindings) = state.bindings.as_mut() {
        for i in HardwareButton::iter() {
            let config_binding = config_profile.bindings.get(&i);
            let state_binding = state_bindings.get(&i);
            if state_binding != config_binding {
                if let Some(binding) = config_binding {
                    state_bindings.insert(i, binding.clone());
                } else {
                    state_bindings.remove(&i);
                }
                let binding = match config_binding {
                    Some(Binding::Preset(preset)) => &preset.entry().binding,
                    Some(Binding::Unknown) => {
                        // Shouldn't occur
                        continue;
                    }
                    None => &[] as &[_],
                };
                let button = Button::new(i as u8, 0, PressType::Normal, binding); // XXX
                send!(worker, WorkerMsg::SetBinding(device_id.clone(), button));
            }
        }
    }

    if let Some(state_left_handed) = state.left_handed.as_mut() {
        if *state_left_handed != config_profile.left_handed {
            *state_left_handed = config_profile.left_handed;
            send!(
                worker,
                WorkerMsg::SetLeftHanded(device_id, config_profile.left_handed)
            );
        }
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

// TODO: format? Multiple files?
// XXX error handling? Don't run `app_data_dir` every save?
pub fn load_config() -> HashMap<String, MouseConfig> {
    let mut path = app_data_dir();
    path.push("config.json");

    let file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            return HashMap::new();
        }
    };

    let mut config: HashMap<String, MouseConfig> = match serde_json::from_reader(file) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Failed to load config: {}", err);
            return HashMap::new();
        }
    };
    // Ensure there are exactly 4 profiles. May change in future.
    for mouse in config.values_mut() {
        mouse.profiles.truncate(4);
        while mouse.profiles.len() < 4 {
            mouse.profiles.push(Profile::default());
        }
    }
    config
}

// TODO: atomic replace
// TODO: don't collect? `SeqIteratorVisitor`
pub fn save_config<'a, T: Iterator<Item = (&'a String, &'a MouseConfig)>>(config: T) {
    let mut path = app_data_dir();
    path.push("config.json");

    let config: HashMap<_, _> = config.collect();

    let file = File::create(&path).unwrap();
    serde_json::to_writer(file, &config).unwrap();
}
