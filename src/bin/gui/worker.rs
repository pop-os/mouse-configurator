use relm4::{ComponentUpdate, Model, Sender};
use std::thread;

use super::AppMsg;
use hp_mouse_configurator::{enumerate, Button, HpMouse};

pub enum WorkerMsg {
    DetectDevice,
    SetDpi(u16),
    SetBinding(Button),
}

pub struct WorkerModel {
    mouse: Option<HpMouse>,
}

impl Model for WorkerModel {
    type Msg = WorkerMsg;
    type Widgets = ();
    type Components = ();
}

fn detect_device() -> Option<HpMouse> {
    for device in enumerate().ok()? {
        eprintln!("Found device: {:?}", device);
        return device.open().ok();
    }
    None
}

impl ComponentUpdate<super::AppModel> for WorkerModel {
    fn init_model(_parent_model: &super::AppModel) -> Self {
        WorkerModel { mouse: None }
    }

    fn update(
        &mut self,
        msg: WorkerMsg,
        _components: &(),
        _sender: Sender<WorkerMsg>,
        parent_sender: Sender<super::AppMsg>,
    ) {
        match msg {
            WorkerMsg::DetectDevice => {
                if let Some(mouse) = detect_device() {
                    // XXX errors
                    let _ = mouse.query_firmware();
                    let _ = mouse.query_battery();
                    let _ = mouse.query_button();
                    let _ = mouse.query_dpi();

                    let events = mouse.read();
                    let parent_sender = parent_sender.clone();

                    thread::spawn(move || {
                        for event in events {
                            if let Ok(event) = event {
                                if let Err(_) = parent_sender.send(AppMsg::Event(event)) {
                                    break;
                                }
                            }
                            // XXX handle error
                        }
                    });

                    self.mouse = Some(mouse);
                }
            }
            WorkerMsg::SetDpi(value) => {
                if let Some(mouse) = &self.mouse {
                    // XXX error
                    let _ = mouse.set_dpi(value);
                }
            }
            WorkerMsg::SetBinding(button) => {
                if let Some(mouse) = &self.mouse {
                    // XXX error
                    let _ = mouse.set_button(button, false);
                }
            }
        }
    }
}
