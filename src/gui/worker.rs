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

use super::{AppMsg, DeviceMonitorProcess};
use hp_mouse_configurator::{Button, Event, HpMouse, HpMouseEvents, ReadRes};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct DeviceId(usize);

// XXX periodically poll for devices? what is done in keyboard configurator?

pub enum WorkerMsg {
    SetDeviceMonitor(DeviceMonitorProcess),
    AddDevice(PathBuf, HpMouse),
    Disconnect(DeviceId),
    SetDpi(DeviceId, u16),
    SetLeftHanded(DeviceId, bool),
    SetBinding(DeviceId, Button),
    HasFirmware(DeviceId),
    Reset(DeviceId),
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
        path: PathBuf,
        mouse: HpMouse,
        sender: &Sender<WorkerMsg>,
        parent_sender: &Sender<super::AppMsg>,
    ) {
        let device_id = self.next_device_id.clone();
        send!(parent_sender, super::AppMsg::DeviceAdded(device_id.clone()));

        let events = mouse.read();
        let running = Arc::new(AtomicBool::new(true));
        thread::spawn(
            glib::clone!(@strong device_id, @strong running, @strong sender, @strong parent_sender => move || {
                reader_thread(device_id, running, events, sender, parent_sender)
            }),
        );

        // XXX errors
        let _ = mouse.query_firmware().unwrap();

        self.devices
            .insert(self.next_device_id.clone(), (path, mouse));
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
            WorkerMsg::SetDeviceMonitor(device_monitor) => {
                thread::spawn(glib::clone!(@strong sender => move || {
                    device_monitor_thread(device_monitor, sender);
                }));
            }
            WorkerMsg::AddDevice(path, mouse) => {
                self.add_device(path, mouse, &sender, &parent_sender);
            }
            WorkerMsg::HasFirmware(id) => {
                // XXX errors
                let mouse = &self.devices.get(&id).unwrap().1;
                let _ = mouse.query_battery().unwrap();
                let _ = mouse.query_button().unwrap();
                let _ = mouse.query_dpi().unwrap();
            }
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
            WorkerMsg::Reset(id) => {
                if let Some((_, mouse)) = &self.devices.get(&id) {
                    // XXX error
                    let _ = mouse.reset();
                }
            }
        }
    }
}

fn device_monitor_thread(device_monitor: DeviceMonitorProcess, sender: Sender<WorkerMsg>) {
    for i in device_monitor {
        // XXX error handling?
        if let Ok((path, mouse)) = i {
            send!(sender, WorkerMsg::AddDevice(path, mouse));
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
                if let Event::Firmware { .. } = &event {
                    send!(sender, WorkerMsg::HasFirmware(device_id.clone()));
                }
                send!(parent_sender, AppMsg::Event(device_id.clone(), event))
            }
            Ok(ReadRes::Continue) => {}
            Err(err) => eprintln!("Error reading event: {}", err), // XXX handle error
        }
    }

    send!(sender, WorkerMsg::Disconnect(device_id));
}
