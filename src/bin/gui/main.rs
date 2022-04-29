use gtk4::{gdk, gdk_pixbuf, gio, glib, pango, prelude::*, subclass::prelude::*};
use relm4::{send, view, AppUpdate, Model, RelmApp, RelmComponent, RelmWorker, Sender, Widgets};
use std::collections::HashMap;

use hp_mouse_configurator::{Button, Event};

mod bindings;
use bindings::{Entry, HardwareButton};
mod dialog;
use dialog::{DialogModel, DialogMsg};
mod worker;
use worker::{WorkerModel, WorkerMsg};

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
    fn add_button(&self, button: &gtk4::LinkButton, x: f64, y: f64, right: bool) {
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
    (0.9, 0.07, true, Some(HardwareButton::Middle)),
    // Left and right click (swapped in left handed mode)
    (0.1, 0.2, false, None),
    (1.0, 0.2, true, Some(HardwareButton::Right)),
    // Scroll buttons
    (0.1, 0.3, false, Some(HardwareButton::ScrollLeft)),
    (1.0, 0.3, true, Some(HardwareButton::ScrollRight)),
    // Side buttons
    (0.0, 0.5, false, Some(HardwareButton::LeftTop)),
    (0.0, 0.6, false, Some(HardwareButton::LeftCenter)),
    (0.0, 0.7, false, Some(HardwareButton::LeftBottom)),
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
                    for (_, _, _, id) in BUTTONS {
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
                                    set_label: watch! { &model.dpi.map_or_else(String::new, |dpi| format!("{}", model.round_dpi(dpi))) },
                                },
                                append: dpi_scale = &gtk4::Scale {
                                    set_hexpand: true,
                                    set_adjustment: &gtk4::Adjustment::new(800., 800., 3000., 50., 50., 0.), // XXX don't hard-code? XXX 800?
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
        buttons: Vec<(Option<HardwareButton>, gtk4::LinkButton)>,
    }

    fn post_init() {
        let mut buttons = Vec::new();

        for (x, y, right, id) in BUTTONS {
            view! {
               button = &gtk4::LinkButton {
                    set_label: "Unknown",
                    add_css_class: "mouse-button",
                    connect_activate_link(sender) => move |_| {
                        send!(sender, AppMsg::SelectButton(*id));
                        gtk4::Inhibit(true)
                    }
                }
            }
            buttons_widget.add_button(&button, *x, *y, *right);
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

    gtk4::init().unwrap();

    /*
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(b"
        .mouse-button {
            background-color: #ff0000;
        }
    ");
    gtk4::StyleContext::add_provider_for_display(&gdk::Display::default().unwrap(), &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    */

    let model = AppModel::default();
    let app = RelmApp::new(model);
    app.run();
}
