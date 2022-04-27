use gtk4::{pango, prelude::*};
use relm4::{send, view, AppUpdate, Model, RelmApp, RelmComponent, RelmWorker, Sender, Widgets};

use hp_mouse_configurator::{Button, Event};

mod bindings;
mod dialog;
use dialog::{DialogModel, DialogMsg};
mod worker;
use worker::{WorkerModel, WorkerMsg};

const BUTTON_RIGHT: u8 = 0;
const BUTTON_MIDDLE: u8 = 1;
const BUTTON_LEFT_BOTTOM: u8 = 2;
const BUTTON_LEFT_TOP: u8 = 3;
const BUTTON_SCROLL_LEFT: u8 = 4;
const BUTTON_SCROLL_RIGHT: u8 = 5;
const BUTTON_LEFT_CENTER: u8 = 6;

static BUTTONS: &[(&str, u8)] = &[
    ("Right Click", BUTTON_RIGHT),
    ("Middle Click", BUTTON_MIDDLE),
    ("Back", BUTTON_LEFT_BOTTOM),
    ("Forward", BUTTON_LEFT_TOP),
    ("Scroll Left", BUTTON_SCROLL_LEFT),
    ("Scroll Right", BUTTON_SCROLL_RIGHT),
    ("Scroll Right", BUTTON_SCROLL_RIGHT),
    ("Super", BUTTON_LEFT_CENTER),
];

struct Mouse {
    min_sensitivity: f64,
    max_sensitivity: f64,
}

static BRAIN_MOUSE: Mouse = Mouse {
    min_sensitivity: 500.,
    max_sensitivity: 3000.,
};

#[derive(relm4::Components)]
struct AppComponents {
    dialog: RelmComponent<DialogModel, AppModel>,
    worker: RelmWorker<WorkerModel, AppModel>,
}

#[derive(Default)]
struct AppModel {
    battery_percent: u8,
    sensitivity: Option<f64>,
}

enum AppMsg {
    RenameConfig,
    Event(Event),
    SetDpi(f64),
    SetBinding(Button),
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = AppComponents;
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: AppMsg, components: &AppComponents, _sender: Sender<AppMsg>) -> bool {
        match msg {
            AppMsg::RenameConfig => {}
            AppMsg::Event(event) => match event {
                Event::Battery { level, .. } => self.battery_percent = level,
                Event::Mouse { dpi, .. } => {
                    if self.sensitivity.is_none() {
                        self.sensitivity = Some(dpi.into());
                    }
                }
                _ => {}
            },
            AppMsg::SetDpi(value) => {
                // XXX don't queue infinitely?
                send!(components.worker, WorkerMsg::SetDpi(value as u16));
                self.sensitivity = Some(value);
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
                        append = &gtk4::Label {
                            set_label: "Configuration",
                        },
                        append = &gtk4::DropDown {
                        },
                        append = &gtk4::MenuButton {
                            set_menu_model: Some(&config_menu),
                            set_icon_name: "view-more-symbolic"
                        }
                    },
                    append: button_box = &gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                    },
                    append = &gtk4::Label {
                        set_label: "Select a button to change its binding. Your settings are automatically saved to firmware.",
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
                            set_sensitive: watch! { model.sensitivity.is_some() },
                            set_selectable: false,
                            set_activatable: false,
                            set_child = Some(&gtk4::Box) {
                                set_orientation: gtk4::Orientation::Horizontal,
                                append = &gtk4::Box {
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
                                append = &gtk4::Scale {
                                    set_hexpand: true,
                                    set_adjustment: &gtk4::Adjustment::new(500., 500., 3000., 1., 1., 1.), // XXX don't hard-code?
                                    set_value: watch! { model.sensitivity.unwrap_or(0.) },
                                    connect_change_value(sender) => move |_, _, value| {
                                        send!(sender, AppMsg::SetDpi(value));
                                        gtk4::Inhibit(false)
                                    }
                                    // add_mark(0., gtk4::PositionType::Bottom, 0.),
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

    fn post_init() {
        for (name, id) in BUTTONS {
            let dialog_sender = components.dialog.sender();
            view! {
               button = &gtk4::Button {
                    set_label: name,
                    connect_clicked => move |_| {
                        send!(dialog_sender, DialogMsg::Show(*id))
                    }
                }
            }
            button_box.append(&button);
        }

        let _ = components.worker.send(WorkerMsg::DetectDevice);
    }
}

relm4::new_action_group!(ConfigActionGroup, "config");
relm4::new_stateless_action!(RenameConfig, ConfigActionGroup, "rename_config");
relm4::new_stateless_action!(ImportConfig, ConfigActionGroup, "import_config");
relm4::new_stateless_action!(ExportConfig, ConfigActionGroup, "export_config");
relm4::new_stateless_action!(ResetConfig, ConfigActionGroup, "reset_config");

fn main() {
    let model = AppModel::default();
    let app = RelmApp::new(model);
    app.run();
}
