use gtk4::{gdk, gdk_pixbuf, gio, glib, pango, prelude::*};
use relm4::{
    actions::{RelmAction, RelmActionGroup},
    send, view, AppUpdate, Model, RelmApp, RelmComponent, RelmWorker, Sender, Widgets,
};
use std::{collections::HashMap, env, process::Command};

use hp_mouse_configurator::Event;

mod bindings;
use bindings::HardwareButton;
mod buttons_widget;
use buttons_widget::{ButtonsWidget, BUTTONS, IMAGE_WIDTH};
mod device_monitor_process;
use device_monitor_process::DeviceMonitorProcess;
mod dialog;
use dialog::{DialogModel, DialogMsg};
mod keycode;
mod profile;
use profile::{apply_profile_diff, load_config, save_config, Binding, MouseConfig, MouseState};
mod swap_button_dialog;
use swap_button_dialog::{SwapButtonDialogModel, SwapButtonDialogMsg};
mod worker;
use worker::{DeviceId, WorkerModel, WorkerMsg};

const DPI_STEP: f64 = 50.;

#[derive(relm4::Components)]
struct AppComponents {
    dialog: RelmComponent<DialogModel, AppModel>,
    swap_button_dialog: RelmComponent<SwapButtonDialogModel, AppModel>,
    worker: RelmWorker<WorkerModel, AppModel>,
}

struct Device {
    id: Option<DeviceId>,
    state: MouseState,
    config: MouseConfig,
}

impl Device {
    fn round_dpi(&self, dpi: f64) -> u16 {
        ((dpi / DPI_STEP).round() * DPI_STEP) as u16
    }

    fn apply_profile_diff(
        &mut self,
        device_id: DeviceId,
        worker: &RelmWorker<WorkerModel, AppModel>,
    ) {
        apply_profile_diff(device_id, &self.config, &mut self.state, worker);
    }

    fn apply_dpi_diff(&mut self, device_id: DeviceId, worker: &RelmWorker<WorkerModel, AppModel>) {
        if let Some(state_dpi) = self.state.dpi {
            let new = self.round_dpi(self.config.info.dpi);
            let old = self.round_dpi(state_dpi);
            if old != new {
                // XXX don't queue infinitely?
                send!(worker, WorkerMsg::SetDpi(device_id, new));
            }
        }
    }
}

struct AppModel {
    devices: Vec<Device>,
    // Index in devices. Must update on remove.
    device_by_id: HashMap<DeviceId, usize>,
    selected_device: Option<usize>,
    bindings_changed: bool,
    device_list_changed: bool,
    device_monitor: Option<DeviceMonitorProcess>,
    show_about: bool,
}

impl AppModel {
    fn new(device_monitor: Option<DeviceMonitorProcess>) -> Self {
        let devices: Vec<_> = load_config()
            .into_iter()
            .map(|config| Device {
                id: None,
                state: MouseState::default(),
                config,
            })
            .collect();
        let selected_device = if devices.len() == 1 { Some(0) } else { None };
        AppModel {
            devices,
            device_by_id: HashMap::new(),
            selected_device,
            bindings_changed: false,
            device_list_changed: false,
            device_monitor,
            show_about: false,
        }
    }

    fn device(&self) -> Option<&Device> {
        Some(&self.devices[self.selected_device?])
    }

    fn device_mut(&mut self) -> Option<&mut Device> {
        Some(&mut self.devices[self.selected_device?])
    }

    fn device_by_id_mut(&mut self, id: &DeviceId) -> Option<&mut Device> {
        Some(&mut self.devices[*self.device_by_id.get(id)?])
    }

    fn add_or_update_device(&mut self, device_id: DeviceId, serial: String) {
        if let Some(idx) = self
            .devices
            .iter()
            .position(|d| d.config.info.serial == serial)
        {
            let mut device = &mut self.devices[idx];
            if let Some(old_id) = &device.id {
                self.device_by_id.remove(&old_id);
            }
            device.state.set_connected();
            device.id = device.id.clone();
            self.device_by_id.insert(device_id.clone(), idx);
        } else {
            let mut device = Device {
                id: Some(device_id.clone()),
                state: MouseState::default(),
                config: MouseConfig::new(serial),
            };
            device.state.set_connected();
            self.devices.push(device);
            let idx = self.devices.len() - 1;
            self.device_by_id.insert(device_id.clone(), idx);
            if idx == 0 {
                self.selected_device = Some(0);
                self.bindings_changed = true;
            }
            self.device_list_changed = true;
        }
    }

    fn remove_device_id(&mut self, id: &DeviceId) {
        if let Some(idx) = self.device_by_id.remove(id) {
            let device = &mut self.devices[idx];
            if device.id.as_ref() == Some(id) {
                device.state.set_disconnected();
                device.id = None;
            }
        }
    }

    // Swap left and right buttons, if in left handed mode
    fn swap_buttons(&self, button: Option<HardwareButton>) -> Option<HardwareButton> {
        if let Some(device) = self.device() {
            if device.config.profile().left_handed && button.is_none() {
                Some(HardwareButton::Right)
            } else if device.config.profile().left_handed && button == Some(HardwareButton::Right) {
                None
            } else {
                button
            }
        } else {
            button
        }
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
    SetBinding(HardwareButton, Binding),
    SelectButton(Option<HardwareButton>),
    SetLeftHanded(bool),
    Reset,
    ShowAbout(bool),
    SelectDevice(Option<usize>),
    SaveConfig,
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = AppComponents;
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: AppMsg, components: &AppComponents, _sender: Sender<AppMsg>) -> bool {
        self.bindings_changed = false;
        self.device_list_changed = false;

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
            AppMsg::DeviceAdded(_id) => {
                // Do nothing until we get `Event::Firmware`
            }
            AppMsg::DeviceRemoved(id) => {
                self.remove_device_id(&id);
            }
            AppMsg::Event(device_id, event) => match event {
                Event::Battery { level, .. } => {
                    let device = self.device_by_id_mut(&device_id).unwrap();
                    device.state.battery_percent = Some(level);
                }
                Event::Mouse {
                    dpi, left_handed, ..
                } => {
                    let device = self.device_by_id_mut(&device_id).unwrap();

                    // Sync dpi from config
                    if device.state.dpi.is_none() {
                        device.state.dpi = Some(dpi.into());
                        device.apply_dpi_diff(device_id.clone(), &components.worker);
                    }

                    // Sync left_handed from config
                    if device.state.left_handed.is_none() {
                        device.state.left_handed = Some(left_handed);
                        device.apply_profile_diff(device_id.clone(), &components.worker);
                    }
                }
                Event::Buttons {
                    buttons, host_id, ..
                } => {
                    let device = self.device_by_id_mut(&device_id).unwrap();
                    if device.state.bindings.is_none() {
                        device.state.set_bindings_from_buttons(host_id, &buttons);
                    }
                }
                Event::Firmware { serial, .. } => {
                    self.add_or_update_device(device_id, serial);
                }
                _ => {}
            },
            AppMsg::SetDpi(value) => {
                if let Some(device) = self.device_mut() {
                    device.config.info.dpi = value;
                    if let Some(device_id) = device.id.clone() {
                        device.apply_dpi_diff(device_id, &components.worker);
                    }
                }
            }
            AppMsg::SelectButton(button) => {
                let button = self.swap_buttons(button);
                if let Some(id) = button {
                    send!(components.dialog, DialogMsg::Show(id))
                } else {
                    let left_handed = self
                        .device()
                        .map_or(false, |x| x.config.profile().left_handed);
                    send!(
                        components.swap_button_dialog,
                        SwapButtonDialogMsg::Show(left_handed)
                    );
                }
            }
            AppMsg::SetBinding(button, binding) => {
                if let Some(device) = self.device_mut() {
                    device.config.profile_mut().bindings.insert(button, binding);
                    if let Some(device_id) = device.id.clone() {
                        device.apply_profile_diff(device_id, &components.worker);
                    }
                    self.bindings_changed = true;
                }
            }
            AppMsg::SetLeftHanded(left_handed) => {
                if let Some(device) = self.device_mut() {
                    device.config.profile_mut().left_handed = left_handed;
                    if let Some(device_id) = device.id.clone() {
                        device.apply_profile_diff(device_id, &components.worker);
                    }
                }
            }
            AppMsg::Reset => {
                if let Some(device) = self.device_mut() {
                    device.config.profile_mut().bindings.clear();
                    device.config.profile_mut().left_handed = false;
                    device.config.info.dpi = 1200.; // XXX depend on device

                    // TODO handle profiles

                    if let Some(device_id) = device.id.clone() {
                        device.apply_profile_diff(device_id.clone(), &components.worker);
                        device.apply_dpi_diff(device_id, &components.worker);
                    }
                }
            }
            AppMsg::ShowAbout(visible) => {
                self.show_about = visible;
            }
            AppMsg::SelectDevice(idx) => {
                if idx != self.selected_device {
                    self.selected_device = idx.filter(|idx| *idx < self.devices.len());
                    self.bindings_changed = true;
                }
            }
            AppMsg::SaveConfig => {
                save_config(self.devices.iter().map(|x| &x.config));
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
                    add_child: device_list_page = &gtk4::ListBox {
                        add_css_class: "frame",
                        set_header_func: header_func,
                        set_halign: gtk4::Align::Center,
                        set_valign: gtk4::Align::Center,
                        set_margin_start: 12,
                        set_margin_end: 12,
                        set_margin_top: 12,
                        set_margin_bottom: 12,
                        connect_row_activated(sender) => move |_, row| {
                            let idx = usize::try_from(row.index()).ok();
                            send!(sender, AppMsg::SelectDevice(idx));
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
                        append = &gtk4::Button {
                            set_halign: gtk4::Align::Center,
                            set_label: "Device List",
                            set_visible: watch! { model.devices.len() > 1 },
                            connect_clicked(sender) => move |_| {
                                send!(sender, AppMsg::SelectDevice(None));
                            }
                        },
                        append = &gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_halign: gtk4::Align::Center,
                            set_spacing: 12,
                            append = &gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 6,
                                set_visible: watch! { model.device().map_or(false, |x| x.state.connected) },
                                append = &gtk4::Image {
                                    set_from_icon_name: Some("battery-symbolic"),
                                },
                                append = &gtk4::Label {
                                    set_label: watch! { &format!("{}%", model.device().and_then(|x| x.state.battery_percent).unwrap_or(0)) }
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
                                        set_label: watch! { &model.device().map_or_else(String::new, |device| format!("{}", device.round_dpi(device.config.info.dpi))) },
                                    },
                                    append: dpi_scale = &gtk4::Scale {
                                        set_hexpand: true,
                                        set_adjustment: &gtk4::Adjustment::new(800., 800., 3000., DPI_STEP, DPI_STEP, 0.), // XXX don't hard-code?
                                        set_value: watch! { model.device().map_or(0., |device| device.config.info.dpi) },
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
            },
            connect_close_request(sender) => move |_| {
                send!(sender, AppMsg::SaveConfig);
                gtk4::Inhibit(false)
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
        first_view_run: bool,
    }

    fn post_init() {
        let first_view_run = true;

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

        glib::timeout_add_seconds(
            10,
            glib::clone!(@strong sender => move || {
                glib::Continue(sender.send(AppMsg::SaveConfig).is_ok())
            }),
        );
    }

    fn post_view() {
        self.about_dialog.set_visible(model.show_about);

        if model.selected_device.is_some() {
            self.stack.set_visible_child(&self.device_page);
        } else if !model.devices.is_empty() {
            self.stack.set_visible_child(&self.device_list_page);
        } else {
            self.stack.set_visible_child(&self.no_device_page);
        }

        if self.first_view_run || model.device_list_changed {
            // Remove existing rows
            while let Some(row) = self.device_list_page.first_child() {
                self.device_list_page.remove(&row);
            }

            // Add new rows
            for device in &model.devices {
                view! {
                    row = gtk4::ListBoxRow {
                        set_selectable: false,
                        set_activatable: true,
                        set_child = Some(&gtk4::Box) {
                            set_orientation: gtk4::Orientation::Vertical,
                            append = &gtk4::Label {
                                set_label: "HP 930 series Creator Wireless Mouse" // TODO don't hard-code
                            },
                            append = &gtk4::Label {
                                set_label: &format!("Serial: {}", device.config.info.serial )
                            }
                        }
                    }
                }
                self.device_list_page.append(&row);
            }
        }

        if self.first_view_run || model.bindings_changed {
            let bindings = model.device().map(|x| &x.config.profile().bindings);
            for (id, button) in &self.buttons {
                if let Some(id) = model.swap_buttons(*id) {
                    button.set_label(
                        &bindings
                            .and_then(|x| x.get(&id))
                            .map_or_else(|| id.def_binding().label.to_string(), |x| x.label()),
                    );
                } else {
                    button.set_label("Left Click");
                }
            }
        }

        self.first_view_run = false;
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

    gio::resources_register_include!("compiled.gresource").unwrap();

    gtk4::init().unwrap();

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

    // TODO
    glib::set_prgname(Some("com.system76.mouseconfigurator"));
    glib::set_application_name("Mouse Configurator");
    let app = gtk4::Application::builder()
        .application_id("com.system76.mouseconfigurator")
        .build();
    app.register(None::<&gio::Cancellable>).unwrap();
    let device_monitor = if !app.is_remote() {
        Some(device_monitor_process::DeviceMonitorProcess::new().unwrap())
    } else {
        None
    };

    let app = RelmApp::with_app(AppModel::new(device_monitor), app);
    app.run();
}

fn header_func(row: &gtk4::ListBoxRow, before: Option<&gtk4::ListBoxRow>) {
    if before.is_none() {
        row.set_header(None::<&gtk4::Widget>)
    } else if row.header().is_none() {
        row.set_header(Some(&gtk4::Separator::new(gtk4::Orientation::Horizontal)));
    }
}
