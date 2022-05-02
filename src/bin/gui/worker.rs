use gtk4::glib;
use nix::{
    errno::Errno,
    poll::{poll, PollFd, PollFlags},
};
use relm4::{send, ComponentUpdate, Model, Sender};
use std::{
    os::unix::io::AsRawFd,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use super::AppMsg;
use hp_mouse_configurator::{enumerate, Button, HpMouse, HpMouseEvents, ReadRes};

pub enum WorkerMsg {
    DetectDevice,
    SetDpi(u16),
    SetLeftHanded(bool),
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

                    let running = Arc::new(AtomicBool::new(true));
                    thread::spawn(
                        glib::clone!(@strong running => move || reader_thread(running, events, parent_sender)),
                    );

                    self.mouse = Some(mouse);
                }
            }
            WorkerMsg::SetDpi(value) => {
                if let Some(mouse) = &self.mouse {
                    // XXX error
                    let _ = mouse.set_dpi(value);
                }
            }
            WorkerMsg::SetLeftHanded(value) => {
                if let Some(mouse) = &self.mouse {
                    // XXX error
                    let _ = mouse.set_left_handed(value);
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

fn reader_thread(
    running: Arc<AtomicBool>,
    mut events: HpMouseEvents,
    parent_sender: Sender<super::AppMsg>,
) {
    while running.load(Ordering::SeqCst) {
        let fd = PollFd::new(events.as_raw_fd(), PollFlags::POLLIN);
        match poll(&mut [fd], 200) {
            Ok(0) | Err(Errno::EINTR) => {
                continue;
            }
            Ok(_) => {}
            Err(err) => panic!("Error polling events: {}", err),
        }

        match events.read() {
            Ok(ReadRes::EOF) => {
                break;
            }
            Ok(ReadRes::Packet(event)) => send!(parent_sender, AppMsg::Event(event)),
            Ok(ReadRes::Continue) => {}
            Err(_err) => {} // XXX handle error
        }
    }
}
