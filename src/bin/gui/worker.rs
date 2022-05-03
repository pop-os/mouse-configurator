use gtk4::glib;
use nix::{
    errno::Errno,
    poll::{poll, PollFd, PollFlags},
};
use relm4::{send, ComponentUpdate, Model, Sender};
use std::{
    collections::HashMap,
    os::unix::io::AsRawFd,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use super::AppMsg;
use hp_mouse_configurator::{enumerate, Button, DeviceInfo, HpMouse, HpMouseEvents, ReadRes};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct DeviceId(usize);

// XXX periodically poll for devices? what is done in keyboard configurator?

pub enum WorkerMsg {
    Disconnect(DeviceId),
    DetectDevices,
    SetDpi(DeviceId, u16),
    SetLeftHanded(DeviceId, bool),
    SetBinding(DeviceId, Button),
}

pub struct WorkerModel {
    next_device_id: DeviceId,
    devices: HashMap<DeviceId, (PathBuf, HpMouse)>, // associate with udev device?
}

impl Model for WorkerModel {
    type Msg = WorkerMsg;
    type Widgets = ();
    type Components = ();
}

impl WorkerModel {
    fn add_device(
        &mut self,
        device: DeviceInfo,
        sender: &Sender<WorkerMsg>,
        parent_sender: &Sender<super::AppMsg>,
    ) {
        let mouse = match device.open() {
            Ok(mouse) => mouse,
            Err(err) => {
                eprintln!("Error opening device: {}", err);
                return;
            }
        };

        // XXX errors
        let _ = mouse.query_firmware();
        let _ = mouse.query_battery();
        let _ = mouse.query_button();
        let _ = mouse.query_dpi();

        let device_id = self.next_device_id.clone();
        send!(parent_sender, super::AppMsg::DeviceAdded(device_id.clone()));

        let events = mouse.read();
        let running = Arc::new(AtomicBool::new(true));
        thread::spawn(
            glib::clone!(@strong device_id, @strong running, @strong sender, @strong parent_sender => move || {
                reader_thread(device_id, running, events, sender, parent_sender)
            }),
        );

        self.devices
            .insert(self.next_device_id.clone(), (device.devnode, mouse));
        self.next_device_id.0 += 1;
    }
}

impl ComponentUpdate<super::AppModel> for WorkerModel {
    fn init_model(_parent_model: &super::AppModel) -> Self {
        WorkerModel {
            next_device_id: DeviceId(0),
            devices: HashMap::new(),
        }
    }

    fn update(
        &mut self,
        msg: WorkerMsg,
        _components: &(),
        sender: Sender<WorkerMsg>,
        parent_sender: Sender<super::AppMsg>,
    ) {
        match msg {
            WorkerMsg::Disconnect(id) => {
                self.devices.remove(&id);
                send!(parent_sender, super::AppMsg::DeviceRemoved(id));
                eprintln!("End reader");
            }
            WorkerMsg::DetectDevices => match enumerate() {
                Ok(devices) => {
                    for device in devices {
                        if !self
                            .devices
                            .values()
                            .any(|(devnode, _)| devnode == &device.devnode)
                        {
                            eprintln!("Found device: {:?}", device);
                            self.add_device(device, &sender, &parent_sender);
                        }
                    }
                }
                Err(err) => eprintln!("Error enumerating devices: {}", err),
            },
            WorkerMsg::SetDpi(id, value) => {
                if let Some((_, mouse)) = &self.devices.get(&id) {
                    // XXX error
                    let _ = mouse.set_dpi(value);
                }
            }
            WorkerMsg::SetLeftHanded(id, value) => {
                if let Some((_, mouse)) = &self.devices.get(&id) {
                    // XXX error
                    let _ = mouse.set_left_handed(value);
                }
            }
            WorkerMsg::SetBinding(id, button) => {
                if let Some((_, mouse)) = &self.devices.get(&id) {
                    // XXX error
                    let _ = mouse.set_button(button, false);
                }
            }
        }
    }
}

fn reader_thread(
    device_id: DeviceId,
    running: Arc<AtomicBool>,
    mut events: HpMouseEvents,
    sender: Sender<WorkerMsg>,
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
            Ok(ReadRes::Packet(event)) => {
                send!(parent_sender, AppMsg::Event(device_id.clone(), event))
            }
            Ok(ReadRes::Continue) => {}
            Err(err) => eprintln!("Error reading event: {}", err), // XXX handle error
        }
    }

    send!(sender, WorkerMsg::Disconnect(device_id));
}
