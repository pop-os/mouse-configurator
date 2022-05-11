use gtk4::{pango, prelude::*};
use relm4::{send, view, ComponentUpdate, Model, Sender, Widgets};
use std::collections::HashMap;

use crate::{
    bindings::{Entry, HardwareButton, BINDINGS},
    profile::Binding,
    AppMsg,
};
use hp_mouse_configurator::Button;

pub enum DialogMsg {
    Show(HardwareButton),
    #[allow(unused)]
    Hide,
    Selected(&'static Entry),
}

pub struct DialogModel {
    button_id: HardwareButton,
    shown: bool,
}

impl Model for DialogModel {
    type Msg = DialogMsg;
    type Widgets = DialogWidgets;
    type Components = ();
}

impl ComponentUpdate<super::AppModel> for DialogModel {
    fn init_model(_parent_model: &super::AppModel) -> Self {
        DialogModel {
            button_id: HardwareButton::Right,
            shown: false,
        }
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
                self.button_id = button_id;
                self.shown = true;
            }
            DialogMsg::Hide => {
                self.shown = false;
            }
            DialogMsg::Selected(entry) => {
                send!(
                    parent_sender,
                    AppMsg::SetBinding(self.button_id, Binding::Preset(entry.id))
                );
                self.shown = false;
            }
        }
    }
}

#[relm4::widget(pub)]
impl Widgets<DialogModel, super::AppModel> for DialogWidgets {
    view! {
        gtk4::Dialog {
            set_transient_for: parent!(Some(&parent_widgets.main_window)),
            set_default_size: args!(300, 300),
            set_modal: true,
            set_hide_on_close: true,
            set_visible: watch!(model.shown),
            set_child = Some(&gtk4::ScrolledWindow) {
                set_hscrollbar_policy: gtk4::PolicyType::Never,
                set_child: vbox = Some(&gtk4::Box) {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_hexpand: true,
                    set_margin_start: 12,
                    set_margin_end: 12,
                    set_margin_top: 12,
                    set_margin_bottom: 12,
                    set_spacing: 6,
                }
            }
        }
    }

    fn post_init() {
        for category in &*BINDINGS {
            let mut rows = HashMap::<gtk4::ListBoxRow, &'static Entry>::new();

            view! {
                label = gtk4::Label {
                    set_label: category.label, // TODO Translate?
                    set_attributes = Some(&pango::AttrList) {
                        insert: pango::AttrInt::new_weight(pango::Weight::Bold)
                    },
                }
            }
            view! {
                list_box = gtk4::ListBox {
                    set_hexpand: true,
                    add_css_class: "frame",
                    set_header_func: header_func,
                }
            }
            vbox.append(&label);
            vbox.append(&list_box);

            for entry in &category.entries {
                view! {
                    row = gtk4::ListBoxRow {
                        set_selectable: false,
                        set_child = Some(&gtk4::Box) {
                            set_margin_top: 6,
                            set_margin_bottom: 6,
                            set_margin_start: 6,
                            set_margin_end: 6,
                            set_orientation: gtk4::Orientation::Horizontal,
                            append = &gtk4::Label {
                                set_label: entry.label, // TODO Translate?
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

fn header_func(row: &gtk4::ListBoxRow, before: Option<&gtk4::ListBoxRow>) {
    if before.is_none() {
        row.set_header(None::<&gtk4::Widget>)
    } else if row.header().is_none() {
        row.set_header(Some(&gtk4::Separator::new(gtk4::Orientation::Horizontal)));
    }
}
