use gtk4::{gio, pango, prelude::*};
use relm4::{send, view, AppUpdate, Model, RelmApp, RelmComponent, RelmWorker, Sender, Widgets};
use std::collections::HashMap;

use hp_mouse_configurator::{Button, Event};

mod bindings;
use bindings::{Entry, HardwareButton};
mod dialog;
use dialog::{DialogModel, DialogMsg};
mod worker;
use worker::{WorkerModel, WorkerMsg};

static BUTTONS: &[(f64, f64, Option<HardwareButton>)] = &[
    (50., 100., None), // Left click button (except in left-handed mode)
    (450., 100., Some(HardwareButton::Right)),
    (350., 50., Some(HardwareButton::Middle)),
    (0., 310., Some(HardwareButton::LeftBottom)),
    (0., 230., Some(HardwareButton::LeftTop)),
    (50., 140., Some(HardwareButton::ScrollLeft)),
    (450., 140., Some(HardwareButton::ScrollRight)),
    (0., 270., Some(HardwareButton::LeftCenter)),
];

#[derive(relm4::Components)]
struct AppComponents {
    dialog: RelmComponent<DialogModel, AppModel>,
    worker: RelmWorker<WorkerModel, AppModel>,
}

#[derive(Default)]
struct AppModel {
    battery_percent: u8,
    dpi: Option<f64>,
    dpi_step: f64,
    bindings: HashMap<HardwareButton, &'static Entry>,
    left_handed: bool,
    bindings_changed: bool,
}

impl AppModel {
    // Swap left and right buttons, if in left handed mode
    fn swap_buttons(&self, button: Option<HardwareButton>) -> Option<HardwareButton> {
        if self.left_handed && button.is_none() {
            Some(HardwareButton::Right)
        } else if self.left_handed && button == Some(HardwareButton::Right) {
            None
        } else {
            button
        }
    }
}

enum AppMsg {
    #[allow(unused)]
    RenameConfig,
    Event(Event),
    SetDpi(f64),
    SetBinding(Button),
    SelectButton(Option<HardwareButton>),
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = AppComponents;
}

impl AppModel {
    fn round_dpi(&self, dpi: f64) -> u16 {
        ((dpi / self.dpi_step).round() * self.dpi_step) as u16
    }
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: AppMsg, components: &AppComponents, _sender: Sender<AppMsg>) -> bool {
        self.bindings_changed = false;

        match msg {
            AppMsg::RenameConfig => {}
            AppMsg::Event(event) => match event {
                Event::Battery { level, .. } => self.battery_percent = level,
                Event::Mouse {
                    dpi,
                    step_dpi,
                    left_handed,
                    ..
                } => {
                    if self.dpi.is_none() {
                        self.dpi = Some(dpi.into());
                        self.dpi_step = step_dpi.into();
                    }
                    self.left_handed = left_handed;
                    self.bindings_changed = true;
                }
                Event::Buttons { buttons, .. } => {
                    // Reset `self.bindings` to defaults
                    self.bindings.clear();
                    for (_, _, id) in BUTTONS {
                        if let Some(id) = id {
                            self.bindings.insert(*id, id.def_binding());
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
                                    self.bindings.insert(id, entry);
                                } else {
                                    self.bindings.remove(&id);
                                    eprintln!("Unrecognized action: {:?}", action);
                                }
                            }
                            Err(err) => {
                                eprintln!("Unable to decode button action: {}", err);
                            }
                        }
                    }

                    self.bindings_changed = true;
                }
                _ => {}
            },
            AppMsg::SetDpi(value) => {
                let new = self.round_dpi(value);
                let old = self.dpi.map(|value| self.round_dpi(value));
                if old != Some(new) {
                    // XXX don't queue infinitely?
                    send!(components.worker, WorkerMsg::SetDpi(new));
                }
                self.dpi = Some(value);
            }
            AppMsg::SelectButton(button) => {
                let button = self.swap_buttons(button);
                if let Some(id) = button {
                    send!(components.dialog, DialogMsg::Show(id as u8))
                } else {
                    // XXX dialog
                    send!(
                        components.worker,
                        WorkerMsg::SetLeftHanded(!self.left_handed)
                    );
                }
            }
            AppMsg::SetBinding(button) => {
                // TODO fewer layers of indirection?
                send!(components.worker, WorkerMsg::SetBinding(button));
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
                set_child = Some(&gtk4::Box) {
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
                                set_label: watch! { &format!("{}%", model.battery_percent) }
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
                        set_hexpand: true,
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
                    append = &gtk4::Overlay {
                        set_child = Some(&gtk4::Image) {
                            set_resource: Some("/org/pop-os/hp-mouse-configurator/mouse-dark.svg"), // XXX light?
                            set_pixel_size: 512,
                        },
                        add_overlay: button_fixed = &gtk4::Fixed {
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
                            set_sensitive: watch! { model.dpi.is_some() },
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
                                    set_label: watch! { &model.dpi.map_or_else(String::new, |dpi| format!("{:.0}", (dpi / 50.).round() * 50.)) },
                                },
                                append: dpi_scale = &gtk4::Scale {
                                    set_hexpand: true,
                                    set_adjustment: &gtk4::Adjustment::new(500., 500., 3000., 50., 50., 0.), // XXX don't hard-code? XXX 800?
                                    set_value: watch! { model.dpi.unwrap_or(0.) },
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

        for (x, y, id) in BUTTONS {
            view! {
               button = &gtk4::Button {
                    set_label: "Unknown",
                    connect_clicked(sender) => move |_| {
                        send!(sender, AppMsg::SelectButton(*id));
                    }
                }
            }
            button_fixed.put(&button, *x, *y);
            buttons.push((*id, button));
        }

        let _ = components.worker.send(WorkerMsg::DetectDevice);
    }

    fn post_view() {
        if model.bindings_changed {
            for (id, button) in &self.buttons {
                if let Some(id) = model.swap_buttons(*id) {
                    button.set_label(model.bindings.get(&id).map_or("Unknown", |x| x.label));
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

    let model = AppModel::default();
    let app = RelmApp::new(model);
    app.run();
}
