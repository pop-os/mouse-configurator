use gtk4::prelude::*;
use relm4::{send, view, ComponentUpdate, Model, Sender, Widgets};
use std::collections::HashMap;

use crate::{
    bindings::{Entry, BINDINGS},
    AppMsg,
};
use hp_mouse_configurator::Button;

pub enum DialogMsg {
    Show(u8),
    Hide,
    Selected(&'static Entry),
}

#[derive(Default)]
pub struct DialogModel {
    button_id: u8,
    shown: bool,
}

impl Model for DialogModel {
    type Msg = DialogMsg;
    type Widgets = DialogWidgets;
    type Components = ();
}

impl ComponentUpdate<super::AppModel> for DialogModel {
    fn init_model(_parent_model: &super::AppModel) -> Self {
        DialogModel::default()
    }

    fn update(
        &mut self,
        msg: DialogMsg,
        _components: &(),
        _sender: Sender<DialogMsg>,
        parent_sender: Sender<AppMsg>,
    ) {
        match msg {
            DialogMsg::Show(button_id) => {
                println!("FOO");
                self.button_id = button_id;
                self.shown = true;
            }
            DialogMsg::Hide => {
                self.shown = false;
            }
            DialogMsg::Selected(entry) => {
                let button = Button::new(self.button_id, 1, 0, entry.binding); // XXX
                send!(parent_sender, AppMsg::SetBinding(button));
            }
        }
    }
}

#[relm4::widget(pub)]
impl Widgets<DialogModel, super::AppModel> for DialogWidgets {
    view! {
        gtk4::Dialog {
            set_transient_for: parent!(Some(&parent_widgets.main_window)),
            set_modal: true,
            set_visible: watch!(model.shown),
            set_child = Some(&gtk4::ScrolledWindow) {
                set_hscrollbar_policy: gtk4::PolicyType::Never,
                set_child: vbox = Some(&gtk4::Box) {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_halign: gtk4::Align::Center,
                    set_hexpand: false,
                    set_margin_start: 12,
                    set_margin_end: 12,
                    set_margin_top: 12,
                    set_margin_bottom: 12,
                }
            }
        }
    }

    fn post_init() {
        for category in BINDINGS {
            let mut rows = HashMap::new();

            view! {
                label = gtk4::Label {
                    set_label: category.label // TODO
                }
            }
            view! {
                list_box = gtk4::ListBox {
                }
            }
            vbox.append(&label);
            vbox.append(&list_box);

            for entry in category.entries {
                // associate with &Entry, or indices?
                view! {
                    row = gtk4::ListBoxRow {
                        set_child = Some(&gtk4::Box) {
                            set_orientation: gtk4::Orientation::Horizontal,
                            append = &gtk4::Label {
                                set_label: entry.label, // TODO
                            }
                        }
                    }
                }
                list_box.append(&row);
                rows.insert(row, entry);
            }

            let sender = sender.clone();
            list_box.connect_row_activated(move |_, row| {
                send!(sender, DialogMsg::Selected(rows.get(row).unwrap()));
            });
        }
    }
}
