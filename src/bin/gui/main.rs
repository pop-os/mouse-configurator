use gtk4::{gdk, gdk_pixbuf, gio, glib, pango, prelude::*, subclass::prelude::*};
use relm4::{send, view, AppUpdate, Model, RelmApp, RelmComponent, RelmWorker, Sender, Widgets};
use std::{collections::HashMap, process::Command};

use hp_mouse_configurator::{Button, Event};

mod bindings;
use bindings::{Entry, HardwareButton};
mod dialog;
use dialog::{DialogModel, DialogMsg};
mod swap_button_dialog;
use swap_button_dialog::{SwapButtonDialogModel, SwapButtonDialogMsg};
mod worker;
use worker::{DeviceId, WorkerModel, WorkerMsg};

#[derive(Default)]
pub struct ButtonsWidgetInner;

#[glib::object_subclass]
impl ObjectSubclass for ButtonsWidgetInner {
    const NAME: &'static str = "ButtonsWidget";
    type Type = ButtonsWidget;
    type ParentType = gtk4::Widget;
}

impl ObjectImpl for ButtonsWidgetInner {}
impl WidgetImpl for ButtonsWidgetInner {}

glib::wrapper! {
    pub struct ButtonsWidget(ObjectSubclass<ButtonsWidgetInner>) @extends gtk4::Widget;
}

impl Default for ButtonsWidget {
    fn default() -> Self {
        let widget = glib::Object::new::<Self>(&[]).unwrap();
        widget.set_layout_manager(Some(&gtk4::ConstraintLayout::new()));
        widget
    }
}

impl ButtonsWidget {
    // XXX RTL?
    fn add_button(&self, button: &gtk4::Button, x: f64, y: f64, right: bool) {
        let w = IMAGE_WIDTH as f64;
        let h = w * IMAGE_RATIO;

        let layout_manager: gtk4::ConstraintLayout =
            self.layout_manager().unwrap().downcast().unwrap();
        button.set_parent(self);
        layout_manager.add_constraint(&gtk4::Constraint::new_constant(
            Some(button),
            gtk4::ConstraintAttribute::Bottom,
            gtk4::ConstraintRelation::Eq,
            y * h,
            0,
        ));
        let side = if right {
            gtk4::ConstraintAttribute::Right
        } else {
            gtk4::ConstraintAttribute::Left
        };
        layout_manager.add_constraint(&gtk4::Constraint::new_constant(
            Some(button),
            side,
            gtk4::ConstraintRelation::Eq,
            x * w,
            0,
        ));
    }
}

const IMAGE_WIDTH: i32 = 512;
const IMAGE_RATIO: f64 = 347. / 474.; // Height/width
static BUTTONS: &[(f64, f64, bool, Option<HardwareButton>)] = &[
    // Middle click
    (0.9, 0.05, true, Some(HardwareButton::Middle)),
    // Left and right click (swapped in left handed mode)
    (0.085, 0.185, false, None),
    (0.998, 0.185, true, Some(HardwareButton::Right)),
    // Scroll buttons
    (0.1, 0.279, false, Some(HardwareButton::ScrollLeft)),
    (0.985, 0.279, true, Some(HardwareButton::ScrollRight)),
    // Side buttons
    (0.0, 0.51, false, Some(HardwareButton::LeftTop)),
    (0.0, 0.597, false, Some(HardwareButton::LeftCenter)),
    (0.0, 0.68, false, Some(HardwareButton::LeftBottom)),
];

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
    bindings: HashMap<HardwareButton, &'static Entry>,
    left_handed: bool,
}

#[derive(Default)]
struct AppModel {
    devices: HashMap<DeviceId, Device>,
    device_id: Option<DeviceId>,
    bindings_changed: bool,
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
            if device.left_handed && button.is_none() {
                Some(HardwareButton::Right)
            } else if device.left_handed && button == Some(HardwareButton::Right) {
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
    Refresh,
    DeviceAdded(DeviceId),
    DeviceRemoved(DeviceId),
    #[allow(unused)]
    RenameConfig,
    Event(DeviceId, Event),
    SetDpi(f64),
    SetBinding(Button),
    SelectButton(Option<HardwareButton>),
    SetLeftHanded(bool),
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = AppComponents;
}

impl AppModel {
    fn round_dpi(&self, dpi: f64) -> u16 {
        let dpi_step = self.device().map_or(1., |x| x.dpi_step);
        ((dpi / dpi_step).round() * dpi_step) as u16
    }
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: AppMsg, components: &AppComponents, _sender: Sender<AppMsg>) -> bool {
        self.bindings_changed = false;

        match msg {
            AppMsg::RenameConfig => {}
            AppMsg::Refresh => {
                send!(components.worker, WorkerMsg::DetectDevices);
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
                    device.left_handed = left_handed;

                    if self.device_id == Some(device_id) {
                        self.bindings_changed = true;
                    }
                }
                Event::Buttons { buttons, .. } => {
                    let bindings = &mut self.devices.get_mut(&device_id).unwrap().bindings;
                    // Reset `self.bindings` to defaults
                    bindings.clear();
                    for (_, _, _, id) in BUTTONS {
                        if let Some(id) = id {
                            bindings.insert(*id, id.def_binding());
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
                                    bindings.insert(id, entry);
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
                    let left_handed = self.device().map_or(false, |x| x.left_handed);
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
        config_menu: {
            "Rename Configuration" => RenameConfig,
            "Import Configuration" => ImportConfig,
            "Export Configuration" => ExportConfig,
            "Reset to Default" => ResetConfig,
        }
    }

    additional_fields! {
        buttons: Vec<(Option<HardwareButton>, gtk4::Button)>,
    }

    fn post_init() {
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

        send!(sender, AppMsg::Refresh);
        glib::timeout_add_seconds_local(
            1,
            glib::clone!(@strong sender => move || {
                glib::Continue(sender.send(AppMsg::Refresh).is_ok())
            }),
        );
    }

    fn post_view() {
        self.stack.set_visible_child(if model.device_id.is_some() {
            &self.device_page
        } else {
            &self.no_device_page
        });

        if model.bindings_changed {
            let bindings = model.device().map(|x| &x.bindings);
            for (id, button) in &self.buttons {
                if let Some(id) = model.swap_buttons(*id) {
                    button.set_label(
                        bindings
                            .and_then(|x| x.get(&id))
                            .map_or("Unknown", |x| x.label),
                    );
                } else {
                    button.set_label("Left Click");
                }
            }
        }
    }
}

relm4::new_action_group!(ConfigActionGroup, "config");
relm4::new_stateless_action!(RenameConfig, ConfigActionGroup, "rename_config");
relm4::new_stateless_action!(ImportConfig, ConfigActionGroup, "import_config");
relm4::new_stateless_action!(ExportConfig, ConfigActionGroup, "export_config");
relm4::new_stateless_action!(ResetConfig, ConfigActionGroup, "reset_config");

fn main() {
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

    let model = AppModel::default();
    let app = RelmApp::new(model);
    app.run();
}
