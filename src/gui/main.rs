use gtk4::{gdk, gdk_pixbuf, gio, glib, pango, prelude::*};
use relm4::{
    actions::{RelmAction, RelmActionGroup},
    send, view, AppUpdate, Model, RelmApp, RelmComponent, RelmWorker, Sender, Widgets,
};
use std::{collections::HashMap, env, process::Command};

use hp_mouse_configurator::{Button, Event};

mod bindings;
use bindings::{Entry, HardwareButton};
mod buttons_widget;
use buttons_widget::{ButtonsWidget, BUTTONS, IMAGE_WIDTH};
mod device_monitor_process;
use device_monitor_process::DeviceMonitorProcess;
mod dialog;
use dialog::{DialogModel, DialogMsg};
mod profile;
use profile::{Binding, Profile};
mod swap_button_dialog;
use swap_button_dialog::{SwapButtonDialogModel, SwapButtonDialogMsg};
mod worker;
use worker::{DeviceId, WorkerModel, WorkerMsg};

#[derive(relm4::Components)]
struct AppComponents {
    dialog: RelmComponent<DialogModel, AppModel>,
    swap_button_dialog: RelmComponent<SwapButtonDialogModel, AppModel>,
    worker: RelmWorker<WorkerModel, AppModel>,
}

#[derive(Default)]
struct Device {
    battery_percent: u8,
    dpi: Option<f64>,
    dpi_step: f64,
    profile: Profile,
}

struct AppModel {
    devices: HashMap<DeviceId, Device>,
    device_id: Option<DeviceId>,
    bindings_changed: bool,
    device_monitor: Option<DeviceMonitorProcess>,
    show_about: bool,
}

impl AppModel {
    fn device(&self) -> Option<&Device> {
        self.devices.get(self.device_id.as_ref()?)
    }

    fn device_mut(&mut self) -> Option<&mut Device> {
        self.devices.get_mut(self.device_id.as_ref()?)
    }

    fn dpi(&self) -> Option<f64> {
        self.device()?.dpi
    }

    // Swap left and right buttons, if in left handed mode
    fn swap_buttons(&self, button: Option<HardwareButton>) -> Option<HardwareButton> {
        if let Some(device) = self.device() {
            if device.profile.left_handed && button.is_none() {
                Some(HardwareButton::Right)
            } else if device.profile.left_handed && button == Some(HardwareButton::Right) {
                None
            } else {
                button
            }
        } else {
            button
        }
    }

    fn round_dpi(&self, dpi: f64) -> u16 {
        let dpi_step = self.device().map_or(1., |x| x.dpi_step);
        ((dpi / dpi_step).round() * dpi_step) as u16
    }
}

enum AppMsg {
    SetDeviceMonitor,
    DeviceAdded(DeviceId),
    DeviceRemoved(DeviceId),
    #[allow(unused)]
    RenameConfig,
    Event(DeviceId, Event),
    SetDpi(f64),
    SetBinding(Button),
    SelectButton(Option<HardwareButton>),
    SetLeftHanded(bool),
    Reset,
    ShowAbout(bool),
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = AppComponents;
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: AppMsg, components: &AppComponents, _sender: Sender<AppMsg>) -> bool {
        self.bindings_changed = false;

        match msg {
            AppMsg::RenameConfig => {}
            AppMsg::SetDeviceMonitor => {
                if let Some(device_monitor) = self.device_monitor.take() {
                    send!(
                        components.worker,
                        WorkerMsg::SetDeviceMonitor(device_monitor)
                    );
                }
            }
            AppMsg::DeviceAdded(id) => {
                self.devices.insert(id.clone(), Device::default());
                self.device_id = Some(id); // XXX
                self.bindings_changed = true;
            }
            AppMsg::DeviceRemoved(id) => {
                self.devices.remove(&id);
                if self.device_id == Some(id) {
                    self.device_id = None;
                }
                self.bindings_changed = true;
            }
            AppMsg::Event(device_id, event) => match event {
                Event::Battery { level, .. } => {
                    let device = self.devices.get_mut(&device_id).unwrap();
                    device.battery_percent = level;
                }
                Event::Mouse {
                    dpi,
                    step_dpi,
                    left_handed,
                    ..
                } => {
                    let device = self.devices.get_mut(&device_id).unwrap();
                    if device.dpi.is_none() {
                        device.dpi = Some(dpi.into());
                        device.dpi_step = step_dpi.into();
                    }
                    device.profile.left_handed = left_handed;

                    if self.device_id == Some(device_id) {
                        self.bindings_changed = true;
                    }
                }
                Event::Buttons { buttons, .. } => {
                    let bindings = &mut self.devices.get_mut(&device_id).unwrap().profile.bindings;
                    // Reset `self.bindings` to defaults
                    bindings.clear();
                    for (_, _, _, id) in BUTTONS {
                        if let Some(id) = id {
                            bindings.insert(*id, Binding::Preset(id.def_binding().id));
                        }
                    }

                    for button in buttons {
                        let id = match HardwareButton::from_u8(button.id) {
                            Some(id) => id,
                            None => {
                                eprintln!("Unrecognized button id: {}", button.id);
                                continue;
                            }
                        };
                        match button.decode_action() {
                            Ok(action) => {
                                if let Some(entry) = Entry::for_binding(&action) {
                                    bindings.insert(id, Binding::Preset(entry.id));
                                } else {
                                    bindings.remove(&id);
                                    eprintln!("Unrecognized action: {:?}", action);
                                }
                            }
                            Err(err) => {
                                eprintln!("Unable to decode button action: {}", err);
                            }
                        }
                    }

                    if self.device_id == Some(device_id) {
                        self.bindings_changed = true;
                    }
                }
                Event::Firmware { .. } => {}
                _ => {}
            },
            AppMsg::SetDpi(value) => {
                let new = self.round_dpi(value);
                let old = self.dpi().map(|value| self.round_dpi(value));
                if old != Some(new) {
                    // XXX don't queue infinitely?
                    if let Some(device_id) = self.device_id.clone() {
                        send!(components.worker, WorkerMsg::SetDpi(device_id, new));
                    }
                }
                if let Some(device) = self.device_mut() {
                    device.dpi = Some(value);
                }
            }
            AppMsg::SelectButton(button) => {
                let button = self.swap_buttons(button);
                if let Some(id) = button {
                    send!(components.dialog, DialogMsg::Show(id as u8))
                } else {
                    let left_handed = self.device().map_or(false, |x| x.profile.left_handed);
                    send!(
                        components.swap_button_dialog,
                        SwapButtonDialogMsg::Show(left_handed)
                    );
                }
            }
            AppMsg::SetBinding(button) => {
                // TODO fewer layers of indirection?
                if let Some(device_id) = self.device_id.clone() {
                    send!(components.worker, WorkerMsg::SetBinding(device_id, button));
                }
            }
            AppMsg::SetLeftHanded(left_handed) => {
                if let Some(device_id) = self.device_id.clone() {
                    send!(
                        components.worker,
                        WorkerMsg::SetLeftHanded(device_id, left_handed)
                    );
                }
            }
            AppMsg::Reset => {
                if let Some(device_id) = self.device_id.clone() {
                    send!(components.worker, WorkerMsg::Reset(device_id));
                }
            }
            AppMsg::ShowAbout(visible) => {
                self.show_about = visible;
            }
        }
        true
    }
}

#[relm4::widget]
impl Widgets<AppModel, ()> for AppWidgets {
    view! {
        main_window = gtk4::ApplicationWindow {
            set_title: Some("HP Mouse"),
            set_default_size: args!(1280, 768),
            set_titlebar = Some(&gtk4::HeaderBar) {
                pack_end = &gtk4::MenuButton {
                    set_menu_model: Some(&menu),
                    set_icon_name: "open-menu-symbolic"
                }
            },
            set_child = Some(&gtk4::ScrolledWindow) {
                set_hscrollbar_policy: gtk4::PolicyType::Never,
                set_child: stack = Some(&gtk4::Stack) {
                    add_child: no_device_page = &gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_halign: gtk4::Align::Center,
                        set_valign: gtk4::Align::Center,
                        set_spacing: 6,
                        append = &gtk4::Image {
                            set_icon_name: Some("input-mouse-symbolic"),
                            set_pixel_size: 128,
                        },
                        append = &gtk4::Label {
                            set_label: "No Mouse Detected",
                            set_attributes = Some(&pango::AttrList) {
                                insert: pango::AttrInt::new_weight(pango::Weight::Bold),
                                insert: pango::AttrFloat::new_scale(pango::SCALE_LARGE)
                            },
                        },
                        append = &gtk4::Label {
                            set_label: "If using USB connection, make sure it is plugged in properly.",
                        },
                        append = &gtk4::LinkButton {
                            set_label: "Check Bluetooth Settings",
                            connect_activate_link => |_| {
                                let _ = Command::new("gnome-control-center").arg("bluetooth").spawn();
                                gtk4::Inhibit(true)
                            }
                        }
                    },
                    add_child: device_page = &gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 6,
                        set_halign: gtk4::Align::Center,
                        set_hexpand: false,
                        set_margin_start: 12,
                        set_margin_end: 12,
                        set_margin_top: 12,
                        set_margin_bottom: 12,
                        append = &gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_halign: gtk4::Align::Center,
                            set_spacing: 12,
                            append = &gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 6,
                                append = &gtk4::Image {
                                    set_from_icon_name: Some("battery-symbolic"),
                                },
                                append = &gtk4::Label {
                                    set_label: watch! { &format!("{}%", model.device().map_or(0, |x| x.battery_percent)) }
                                },
                            },
                            append = &gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 6,
                                append = &gtk4::Image {
                                    set_from_icon_name: Some("help-info-symbolic"),
                                },
                                append = &gtk4::Label {
                                    set_label: "About This Mouse",
                                }
                            },
                        },
                        append = &gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            append = &gtk4::Label {
                                set_label: "Configuration",
                            },
                            append = &gtk4::DropDown {
                                set_hexpand: true,
                            },
                            append = &gtk4::MenuButton {
                                set_menu_model: Some(&config_menu),
                                set_icon_name: "view-more-symbolic"
                            }
                        },
                        // One element box to work around weird size allocation behavior
                        append = &gtk4::Box {
                            set_margin_top: 6,
                            set_margin_bottom: 6,
                            set_vexpand: false,
                            set_halign: gtk4::Align::Center,
                            append = &gtk4::Overlay {
                                set_child = Some(&gtk4::Picture) {
                                    set_pixbuf: Some(&gdk_pixbuf::Pixbuf::from_resource_at_scale("/org/pop-os/hp-mouse-configurator/mouse-dark.svg", IMAGE_WIDTH, -1, true).unwrap()), // XXX light
                                    set_can_shrink: false,
                                },
                                add_overlay: buttons_widget = &ButtonsWidget {
                                },
                                set_measure_overlay: args!(&buttons_widget, false),
                            }
                        },
                        append = &gtk4::Label {
                            set_label: "Select a button to change its binding. Your settings are automatically saved to firmware.",
                            set_margin_bottom: 12,
                        },
                        append = &gtk4::Label {
                            set_label: "Sensitivity",
                            set_attributes = Some(&pango::AttrList) {
                                insert: pango::AttrInt::new_weight(pango::Weight::Bold)
                            },
                            set_halign: gtk4::Align::Start
                        },
                        append = &gtk4::ListBox {
                            add_css_class: "frame",
                            append = &gtk4::ListBoxRow {
                                set_sensitive: watch! { model.dpi().is_some() },
                                set_selectable: false,
                                set_activatable: false,
                                set_child = Some(&gtk4::Box) {
                                    set_orientation: gtk4::Orientation::Horizontal,
                                    append = &gtk4::Box {
                                        set_margin_top: 6,
                                        set_margin_bottom: 6,
                                        set_margin_start: 6,
                                        set_margin_end: 6,
                                        set_orientation: gtk4::Orientation::Vertical,
                                        append = &gtk4::Label {
                                            set_label: "Mouse Cursor Speed",
                                            set_attributes = Some(&pango::AttrList) {
                                                insert: pango::AttrInt::new_weight(pango::Weight::Bold)
                                            }
                                        },
                                        append = &gtk4::Label {
                                            set_label: "Sensitivity (DPI)",
                                        }
                                    },
                                    append = &gtk4::Label {
                                        set_label: watch! { &model.dpi().map_or_else(String::new, |dpi| format!("{}", model.round_dpi(dpi))) },
                                    },
                                    append: dpi_scale = &gtk4::Scale {
                                        set_hexpand: true,
                                        set_adjustment: &gtk4::Adjustment::new(800., 800., 3000., 50., 50., 0.), // XXX don't hard-code? XXX 800?
                                        set_value: watch! { model.dpi().unwrap_or(0.) },
                                        connect_change_value(sender) => move |_, _, value| {
                                            send!(sender, AppMsg::SetDpi(value));
                                            gtk4::Inhibit(false)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    menu! {
        menu: {
            "Reset to Default" => ResetAction,
            "About" => AboutAction,
        },
        config_menu: {
            "Rename Configuration" => RenameConfig,
            "Import Configuration" => ImportConfig,
            "Export Configuration" => ExportConfig,
        }
    }

    additional_fields! {
        buttons: Vec<(Option<HardwareButton>, gtk4::Button)>,
        about_dialog: gtk4::AboutDialog,
    }

    fn post_init() {
        let about_dialog = gtk4::AboutDialog::builder()
            .transient_for(&main_window)
            .hide_on_close(true)
            .version(env!("CARGO_PKG_VERSION"))
            .logo_icon_name("input-mouse-symbolic") // TODO
            .copyright("Copyright 2022 Hewlett-Packard Development Company, L.P.")
            .license_type(gtk4::License::MitX11)
            .build();
        about_dialog.connect_close_request(glib::clone!(@strong sender => move |_| {
            send!(sender, AppMsg::ShowAbout(false));
            gtk4::Inhibit(true)
        }));

        let mut buttons = Vec::new();

        for (x, y, right, id) in BUTTONS {
            view! {
               button = &gtk4::Button {
                    set_label: "Unknown",
                    add_css_class: "mouse-button",
                    add_css_class: "flat",
                    connect_clicked(sender) => move |_| {
                        send!(sender, AppMsg::SelectButton(*id));
                    }
                }
            }
            buttons_widget.add_button(&button, *x, *y, *right);
            buttons.push((*id, button));
        }

        let app_group = RelmActionGroup::<AppActionGroup>::new();
        let device_group = RelmActionGroup::<DeviceActionGroup>::new();

        let about_action: RelmAction<AboutAction> =
            RelmAction::new_stateless(glib::clone!(@strong sender => move |_| {
                send!(sender, AppMsg::ShowAbout(true));
            }));
        app_group.add_action(about_action);

        let reset_action: RelmAction<ResetAction> =
            RelmAction::new_stateless(glib::clone!(@strong sender => move |_| {
                send!(sender, AppMsg::Reset);
            }));
        device_group.add_action(reset_action);

        let app_actions = app_group.into_action_group();
        let device_actions = device_group.into_action_group();
        main_window.insert_action_group("app", Some(&app_actions));
        main_window.insert_action_group("device", Some(&device_actions));

        send!(sender, AppMsg::SetDeviceMonitor);
    }

    fn post_view() {
        self.about_dialog.set_visible(model.show_about);

        self.stack.set_visible_child(if model.device_id.is_some() {
            &self.device_page
        } else {
            &self.no_device_page
        });

        if model.bindings_changed {
            let bindings = model.device().map(|x| &x.profile.bindings);
            for (id, button) in &self.buttons {
                if let Some(id) = model.swap_buttons(*id) {
                    button.set_label(
                        &bindings
                            .and_then(|x| x.get(&id))
                            .map_or_else(|| "Unknown".to_string(), |x| x.label()),
                    );
                } else {
                    button.set_label("Left Click");
                }
            }
        }
    }
}

relm4::new_action_group!(AppActionGroup, "app");
relm4::new_stateless_action!(AboutAction, AppActionGroup, "about");

relm4::new_action_group!(DeviceActionGroup, "device");
relm4::new_stateless_action!(ResetAction, DeviceActionGroup, "reset_config");

relm4::new_action_group!(ConfigActionGroup, "config");
relm4::new_stateless_action!(RenameConfig, ConfigActionGroup, "rename_config");
relm4::new_stateless_action!(ImportConfig, ConfigActionGroup, "import_config");
relm4::new_stateless_action!(ExportConfig, ConfigActionGroup, "export_config");

fn main() {
    let mut args = env::args().skip(1);
    if args.next().as_deref() == Some("--device-monitor") {
        device_monitor_process::device_monitor_process();
        return;
    }

    let device_monitor = device_monitor_process::DeviceMonitorProcess::new().unwrap();

    gio::resources_register_include!("compiled.gresource").unwrap();

    gtk4::init().unwrap();

    // TODO
    glib::set_prgname(Some("com.system76.mouseconfigurator"));
    glib::set_application_name("Mouse Configurator");

    let provider = gtk4::CssProvider::new();
    provider.load_from_data(
        b"
        .mouse-button {
            /* background-color: #ff0000; */
            padding: 0;
        }
    ",
    );
    gtk4::StyleContext::add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let model = AppModel {
        devices: HashMap::new(),
        device_id: None,
        bindings_changed: false,
        device_monitor: Some(device_monitor),
        show_about: false,
    };
    let app = RelmApp::new(model);
    app.run();
}
