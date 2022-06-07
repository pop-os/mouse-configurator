use gtk4::{gdk, gdk_pixbuf, gio, glib, pango, prelude::*};
use relm4::{
    actions::{RelmAction, RelmActionGroup},
    send, view, AppUpdate, Model, RelmApp, RelmComponent, RelmWorker, Sender, Widgets,
};
use std::{collections::HashMap, env, path::PathBuf, process::Command};

use mouse_configurator::Event;

mod bindings;
use bindings::HardwareButton;
mod binding_dialog;
use binding_dialog::{BindingDialogModel, BindingDialogMsg};
mod buttons_widget;
use buttons_widget::{ButtonsWidget, BUTTONS, IMAGE_WIDTH};
mod device_monitor_process;
use device_monitor_process::DeviceMonitorProcess;
mod dialogs;
use dialogs::*;
mod keycode;
mod profile;
use profile::{
    apply_profile_diff, load_config, save_config, Binding, MouseConfig, MouseState, Profile,
};
mod swap_button_dialog;
use swap_button_dialog::{SwapButtonDialogModel, SwapButtonDialogMsg};
mod util;
mod worker;
use worker::{DeviceId, WorkerModel, WorkerMsg};

const DPI_STEP: f64 = 50.;

#[derive(relm4::Components)]
struct AppComponents {
    dialog: RelmComponent<BindingDialogModel, AppModel>,
    swap_button_dialog: RelmComponent<SwapButtonDialogModel, AppModel>,
    worker: RelmWorker<WorkerModel, AppModel>,
}

struct Device {
    id: Option<DeviceId>,
    state: MouseState,
    config: MouseConfig,
    serial: String,
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
            let new = self.round_dpi(self.config.dpi);
            let old = self.round_dpi(state_dpi);
            if old != new {
                // XXX don't queue infinitely?
                send!(worker, WorkerMsg::SetDpi(device_id, new));
            }
        }
    }
}

#[derive(Default)]
struct AppModel {
    devices: Vec<Device>,
    // Index in devices. Must update on remove.
    device_by_id: HashMap<DeviceId, usize>,
    selected_device: Option<usize>,
    bindings_changed: bool,
    device_list_changed: bool,
    profiles_changed: bool,
    show_about_mouse: bool,
    rename_config: bool,
    device_monitor: Option<DeviceMonitorProcess>,
    error: Option<String>,
}

impl AppModel {
    fn new(device_monitor: Option<DeviceMonitorProcess>) -> Self {
        let devices: Vec<_> = load_config()
            .into_iter()
            .map(|(serial, config)| Device {
                id: None,
                state: MouseState::default(),
                config,
                serial,
            })
            .collect();
        let selected_device = if devices.len() == 1 { Some(0) } else { None };
        AppModel {
            devices,
            selected_device,
            device_monitor,
            ..Default::default()
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

    fn add_or_update_device(
        &mut self,
        device_id: DeviceId,
        device: String,
        serial: String,
        version: (u16, u16, u16),
    ) {
        if let Some(idx) = self.devices.iter().position(|d| d.serial == serial) {
            let mut device = &mut self.devices[idx];
            if let Some(old_id) = &device.id {
                self.device_by_id.remove(&old_id);
            }
            device.state.set_connected();
            device.state.firmware_version = Some(version);
            device.id = Some(device_id.clone());
            self.device_by_id.insert(device_id.clone(), idx);
        } else {
            let mut device = Device {
                id: Some(device_id.clone()),
                state: MouseState::default(),
                config: MouseConfig::new(device),
                serial,
            };
            device.state.set_connected();
            device.state.firmware_version = Some(version);
            device.id = Some(device_id.clone());
            self.devices.push(device);
            let idx = self.devices.len() - 1;
            self.device_by_id.insert(device_id.clone(), idx);
            if idx == 0 {
                self.set_selected_device(Some(0));
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

    fn remove_device(&mut self, device: usize) {
        self.devices.remove(device);

        self.device_by_id = self
            .device_by_id
            .drain()
            .filter_map(|(id, d)| {
                if d == device {
                    None
                } else if d > device {
                    Some((id, d - 1))
                } else {
                    Some((id, d))
                }
            })
            .collect();

        if let Some(selected_device) = self.selected_device {
            if selected_device == device {
                self.set_selected_device(None);
            } else if selected_device > device {
                self.set_selected_device(Some(selected_device - 1));
            }
        }

        self.device_list_changed = true;
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

    fn set_selected_device(&mut self, selected_device: Option<usize>) {
        self.selected_device = selected_device.filter(|x| *x < self.devices.len());
        self.bindings_changed = true;
        self.profiles_changed = true;
    }
}

enum AppMsg {
    SetDeviceMonitor,
    DeviceAdded(DeviceId),
    DeviceRemoved(DeviceId),
    ToggleRenameConfig,
    RenameConfig(Option<String>),
    Event(DeviceId, Event),
    SetDpi(f64),
    SetBinding(HardwareButton, Binding),
    SelectButton(Option<HardwareButton>),
    SetLeftHanded(bool),
    Reset,
    Remove,
    SelectDevice(Option<usize>),
    SaveConfig,
    ShowAboutMouse,
    SelectProfile(usize),
    ExportConfig(PathBuf),
    ImportConfig(PathBuf),
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
        self.show_about_mouse = false;
        self.profiles_changed = false;
        self.error = None;

        match msg {
            AppMsg::ToggleRenameConfig => {
                self.rename_config = !self.rename_config;
            }
            AppMsg::RenameConfig(name) => {
                if let Some(device) = self.device_mut() {
                    device.config.profile_mut().name = name;
                    self.profiles_changed = true;
                }
            }
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
                Event::Firmware {
                    device,
                    serial,
                    version,
                } => {
                    self.add_or_update_device(device_id, device, serial, version);
                }
                _ => {}
            },
            AppMsg::SetDpi(value) => {
                if let Some(device) = self.device_mut() {
                    device.config.dpi = value;
                    if let Some(device_id) = device.id.clone() {
                        device.apply_dpi_diff(device_id, &components.worker);
                    }
                }
            }
            AppMsg::SelectButton(button) => {
                let button = self.swap_buttons(button);
                if let Some(id) = button {
                    send!(components.dialog, BindingDialogMsg::Show(id))
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
                    if binding == Binding::Preset(button.def_binding().id) {
                        device.config.profile_mut().bindings.remove(&button);
                    } else {
                        device.config.profile_mut().bindings.insert(button, binding);
                    }
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
                    self.bindings_changed = true;
                }
            }
            AppMsg::Reset => {
                if let Some(device) = self.device_mut() {
                    for profile in device.config.profiles_mut() {
                        *profile = Profile::default();
                    }
                    device.config.dpi = 1200.; // XXX depend on device

                    if let Some(device_id) = device.id.clone() {
                        device.apply_profile_diff(device_id.clone(), &components.worker);
                        device.apply_dpi_diff(device_id, &components.worker);
                    }

                    self.bindings_changed = true;
                }
            }
            AppMsg::Remove => {
                if let Some(device) = self.selected_device {
                    self.remove_device(device);
                }
            }
            AppMsg::SelectDevice(idx) => {
                if idx != self.selected_device {
                    self.set_selected_device(idx);
                }
            }
            AppMsg::SaveConfig => {
                save_config(self.devices.iter().map(|x| (&x.serial, &x.config)));
            }
            AppMsg::ShowAboutMouse => {
                self.show_about_mouse = true;
            }
            AppMsg::SelectProfile(profile) => {
                if let Some(device) = self.device_mut() {
                    if profile != device.config.profile_num()
                        && profile < device.config.profiles().len()
                    {
                        device.config.select_profile(profile);
                        if let Some(device_id) = device.id.clone() {
                            device.apply_profile_diff(device_id, &components.worker);
                        }
                        self.profiles_changed = true;
                        self.bindings_changed = true;
                    }
                }
            }
            AppMsg::ImportConfig(path) => {
                if let Some(device) = self.device_mut() {
                    match MouseConfig::import(&path) {
                        Ok(config) => {
                            device.config = config;
                        }
                        Err(err) => {
                            self.error = Some(format!("Failed to import config: {}", err));
                        }
                    }
                }
            }
            AppMsg::ExportConfig(path) => {
                if let Some(device) = self.device_mut() {
                    match device.config.export(&path) {
                        Ok(()) => {}
                        Err(err) => {
                            self.error = Some(format!("Failed to export config: {}", err));
                        }
                    }
                }
            }
        }
        true
    }
}

#[relm4::widget]
impl Widgets<AppModel, ()> for AppWidgets {
    view! {
        main_window = gtk4::ApplicationWindow {
            set_title: Some("Mouse Configurator"),
            set_default_size: args!(1280, 768),
            set_titlebar = Some(&gtk4::HeaderBar) {
                pack_start = &gtk4::Button {
                    add_css_class: "flat",
                    set_visible: watch! { model.selected_device.is_some() && model.devices.len() > 1 },
                    set_icon_name: "go-previous-symbolic",
                    connect_clicked(sender) => move |_| {
                        send!(sender, AppMsg::SelectDevice(None));
                    }
                },
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
                        set_header_func: util::header_func,
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
                        set_spacing: 18,
                        set_halign: gtk4::Align::Center,
                        set_hexpand: false,
                        set_margin_start: 12,
                        set_margin_end: 12,
                        set_margin_top: 18,
                        set_margin_bottom: 36,
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
                            append = &gtk4::Button {
                                add_css_class: "flat",
                                set_child = Some(&gtk4::Box) {
                                    set_orientation: gtk4::Orientation::Horizontal,
                                    set_spacing: 6,
                                    append = &gtk4::Image {
                                        set_from_icon_name: Some("help-info-symbolic"),
                                    },
                                    append = &gtk4::Label {
                                        set_label: "About This Mouse",
                                    }
                                },
                                connect_clicked(sender) => move |_| send!(sender, AppMsg::ShowAboutMouse)
                            },
                        },
                        append = &gtk4::Box {
                            set_margin_bottom: 18,
                            set_spacing: 8,
                            set_orientation: gtk4::Orientation::Horizontal,
                            append = &gtk4::Label {
                                set_label: "Configuration",
                            },
                            append: profiles_stack = &gtk4::Stack {
                                add_child: profiles_dropdown = &gtk4::DropDown {
                                    set_hexpand: true,
                                    // set_show_arrow: false, XXX requires GTK 4.6?
                                },
                                add_child: profiles_entry = &gtk4::Entry {
                                    set_max_length: 30,
                                    connect_activate(sender) => move |_| {
                                        send!(sender, AppMsg::ToggleRenameConfig);
                                    }
                                }

                            },
                            append: rename_button = &gtk4::Button {
                                connect_clicked(sender) => move |_| {
                                    send!(sender, AppMsg::ToggleRenameConfig);
                                }
                            }
                        },
                        // One element box to work around weird size allocation behavior
                        append = &gtk4::Box {
                            set_margin_top: 6,
                            set_margin_bottom: 6,
                            set_vexpand: false,
                            set_halign: gtk4::Align::Center,
                            append = &gtk4::Overlay {
                                set_child: mouse_picture = Some(&gtk4::Picture) {
                                    set_can_shrink: false,
                                },
                                add_overlay: buttons_widget = &ButtonsWidget {
                                },
                                set_measure_overlay: args!(&buttons_widget, false),
                            }
                        },
                        append = &gtk4::Label {
                            set_label: "Select a button to change its binding. Your settings are automatically saved to firmware.",
                            set_margin_bottom: 18,
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
                                    set_margin_top: 6,
                                    set_margin_bottom: 6,
                                    set_margin_start: 6,
                                    set_margin_end: 6,
                                    append = &gtk4::Box {
                                        set_margin_end: 36,
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
                                        set_label: watch! { &model.device().map_or_else(String::new, |device| format!("{}", device.round_dpi(device.config.dpi))) },
                                    },
                                    append: dpi_scale = &gtk4::Scale {
                                        set_hexpand: true,
                                        set_adjustment: &gtk4::Adjustment::new(800., 800., 3000., DPI_STEP, DPI_STEP, 0.), // XXX don't hard-code?
                                        set_value: watch! { model.device().map_or(0., |device| device.config.dpi) },
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
            "Import Configuration" => ImportConfig,
            "Export Configuration" => ExportConfig,
            "Reset to Default" => ResetAction,
            "Remove Device" => RemoveAction,
            "About" => AboutAction,
        }
    }

    additional_fields! {
        buttons: Vec<(Option<HardwareButton>, gtk4::Button)>,
        first_view_run: bool,
        desktop_settings: gio::Settings,
        device_actions: gio::SimpleActionGroup,
        profiles_dropdown_signal: glib::SignalHandlerId,
    }

    fn post_init() {
        let first_view_run = true;

        let profiles_dropdown_signal = profiles_dropdown.connect_selected_notify(
            glib::clone!(@strong sender => move |drop_down| {
                send!(sender, AppMsg::SelectProfile(drop_down.selected() as usize));
            }),
        );

        // Detect dark/light theme
        fn update_theme(desktop_settings: &gio::Settings, mouse_picture: &gtk4::Picture) {
            let resource = if desktop_settings
                .string("gtk-theme")
                .as_str()
                .contains("dark")
            {
                "/org/pop-os/mouse-configurator/mouse-dark.svg"
            } else {
                "/org/pop-os/mouse-configurator/mouse-light.svg"
            };
            mouse_picture.set_pixbuf(Some(
                &gdk_pixbuf::Pixbuf::from_resource_at_scale(resource, IMAGE_WIDTH, -1, true)
                    .unwrap(),
            ));
        }
        let desktop_settings = gio::Settings::new("org.gnome.desktop.interface");
        desktop_settings.connect_changed(
            Some("gtk-theme"),
            glib::clone!(@strong mouse_picture => move |desktop_settings, _| {
                update_theme(desktop_settings, &mouse_picture);
            }),
        );
        update_theme(&desktop_settings, &mouse_picture);

        let mut buttons = Vec::new();

        for (x, y, right, id) in BUTTONS {
            view! {
               button = &gtk4::Button {
                    set_margin_start: 8,
                    set_margin_end: 8,
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
            RelmAction::new_stateless(glib::clone!(@weak main_window => move |_| {
                show_about_dialog(&main_window);
            }));
        app_group.add_action(about_action);

        let import_action: RelmAction<ImportConfig> = RelmAction::new_stateless(
            glib::clone!(@strong main_window, @strong sender => move |_| {
                show_import_dialog(&main_window, glib::clone!(@strong sender => move |path| {
                    send!(sender, AppMsg::ImportConfig(path));
                }));
            }),
        );
        device_group.add_action(import_action);
        let export_action: RelmAction<ExportConfig> = RelmAction::new_stateless(
            glib::clone!(@strong main_window, @strong sender => move |_| {
                show_export_dialog(&main_window, glib::clone!(@strong sender => move |path| {
                    send!(sender, AppMsg::ExportConfig(path));
                }));
            }),
        );
        device_group.add_action(export_action);
        let reset_action: RelmAction<ResetAction> = RelmAction::new_stateless(
            glib::clone!(@strong main_window, @strong sender => move |_| {
                show_prompt_dialog(&main_window, "Reset sensitivity and all configurations for this device?",
                    glib::clone!(@strong sender => move || {
                        send!(sender, AppMsg::Reset);
                    }));
            }),
        );
        // XXX Only show, make sensitive when device not connected?
        device_group.add_action(reset_action);
        let remove_action: RelmAction<RemoveAction> = RelmAction::new_stateless(
            glib::clone!(@strong main_window, @strong sender => move |_| {
                show_prompt_dialog(&main_window, "Remove device and saved configurations?",
                    glib::clone!(@strong sender => move || {
                        send!(sender, AppMsg::Remove);
                    }));
            }),
        );
        device_group.add_action(remove_action);

        let app_actions = app_group.into_action_group();
        let device_actions = device_group.into_action_group();
        main_window.insert_action_group("app", Some(&app_actions));

        send!(sender, AppMsg::SetDeviceMonitor);

        glib::timeout_add_seconds(
            10,
            glib::clone!(@strong sender => move || {
                glib::Continue(sender.send(AppMsg::SaveConfig).is_ok())
            }),
        );
    }

    fn post_view() {
        if let Some(error) = model.error.as_ref() {
            show_error_dialog(&main_window, error);
        }

        if model.selected_device.is_some() {
            let connected = model.device().map_or(false, |x| x.state.connected);
            self.device_actions
                .lookup_action("remove")
                .unwrap()
                .downcast_ref::<gio::SimpleAction>()
                .unwrap()
                .set_enabled(!connected);

            self.stack.set_visible_child(&self.device_page);
            let in_rename_config = self.profiles_stack.visible_child().as_ref()
                == Some(self.profiles_entry.upcast_ref::<gtk4::Widget>());
            if model.rename_config {
                rename_button.set_icon_name("emblem-ok-symbolic");
                if !in_rename_config {
                    let text = model
                        .device()
                        .and_then(|x| x.config.profile().name.as_deref())
                        .unwrap_or("");
                    self.profiles_entry.set_text(text);
                    self.profiles_stack.set_visible_child(&self.profiles_entry);
                    self.profiles_entry.grab_focus();
                }
            } else {
                rename_button.set_icon_name("document-edit-symbolic");
                if in_rename_config {
                    let text = self.profiles_entry.text();
                    let name = if text.is_empty() {
                        None
                    } else {
                        Some(text.as_str().to_string())
                    };
                    send!(sender, AppMsg::RenameConfig(name));
                    self.profiles_stack
                        .set_visible_child(&self.profiles_dropdown);
                }
            }
            main_window.insert_action_group("device", Some(&self.device_actions));
        } else if !model.devices.is_empty() {
            self.stack.set_visible_child(&self.device_list_page);
            main_window.insert_action_group("device", None::<&gio::ActionGroup>);
        } else {
            self.stack.set_visible_child(&self.no_device_page);
            main_window.insert_action_group("device", None::<&gio::ActionGroup>);
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
                                set_label: &format!("Unique ID: {}", device.serial )
                            }
                        }
                    }
                }
                self.device_list_page.append(&row);
            }
        }

        if let Some(device) = model.device() {
            if self.first_view_run || model.bindings_changed {
                let bindings = &device.config.profile().bindings;
                for (id, button) in &self.buttons {
                    if let Some(id) = model.swap_buttons(*id) {
                        button.set_label(
                            &bindings
                                .get(&id)
                                .map_or_else(|| id.def_binding().label.to_string(), |x| x.label()),
                        );
                    } else {
                        button.set_label("Left Click");
                    }
                }
            }

            if self.first_view_run || model.profiles_changed {
                let default_labels = &[
                    "Configuration One",
                    "Configuration Two",
                    "Configuration Three",
                    "Configuration Four",
                ];
                let labels: Vec<_> = device
                    .config
                    .profiles()
                    .iter()
                    .enumerate()
                    .map(|(n, profile)| profile.name.as_deref().unwrap_or(default_labels[n]))
                    .collect();
                self.profiles_dropdown
                    .block_signal(&self.profiles_dropdown_signal);
                self.profiles_dropdown
                    .set_model(Some(&gtk4::StringList::new(&labels)));
                self.profiles_dropdown
                    .set_selected(device.config.profile_num() as u32);
                self.profiles_dropdown
                    .unblock_signal(&self.profiles_dropdown_signal);
            }

            if model.show_about_mouse {
                show_info_dialog(
                    &main_window,
                    &device.config.device,
                    &device.serial,
                    device.state.firmware_version,
                );
            }
        }

        self.first_view_run = false;
    }
}

relm4::new_action_group!(AppActionGroup, "app");
relm4::new_stateless_action!(AboutAction, AppActionGroup, "about");

relm4::new_action_group!(DeviceActionGroup, "device");
relm4::new_stateless_action!(ImportConfig, DeviceActionGroup, "import_config");
relm4::new_stateless_action!(ExportConfig, DeviceActionGroup, "export_config");
relm4::new_stateless_action!(ResetAction, DeviceActionGroup, "reset_config");
relm4::new_stateless_action!(RemoveAction, DeviceActionGroup, "remove");

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--device-monitor") => {
            device_monitor_process::device_monitor_process();
            return;
        }
        Some("--add-fake-device") => {
            let mut configs = load_config();
            let serial = format!("FAKE{:16X}", rand::random::<u64>());
            configs.insert(serial, MouseConfig::new("Brain".to_string()));
            save_config(configs.iter());
        }
        _ => {}
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

    glib::set_prgname(Some("org.pop_os.mouseconfigurator"));
    glib::set_application_name("Mouse Configurator");
    let app = gtk4::Application::builder()
        .application_id("org.pop_os.mouseconfigurator")
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
