use gtk4::{pango, prelude::*};
use relm4::{send, view, ComponentUpdate, Model, Sender, Widgets};
use std::{cell::Cell, collections::HashMap, ptr, rc::Rc};

use crate::{
    bindings::{Category, Entry, HardwareButton, BINDINGS},
    profile::Binding,
    util, AppMsg,
};

pub enum BindingDialogMsg {
    Show(HardwareButton),
    #[allow(unused)]
    Hide,
    SelectCategory(Option<&'static Category>),
    Selected(&'static Entry),
}

pub struct BindingDialogModel {
    button_id: HardwareButton,
    category: Option<&'static Category>,
    shown: bool,
}

impl Model for BindingDialogModel {
    type Msg = BindingDialogMsg;
    type Widgets = BindingDialogWidgets;
    type Components = ();
}

impl ComponentUpdate<super::AppModel> for BindingDialogModel {
    fn init_model(_parent_model: &super::AppModel) -> Self {
        BindingDialogModel {
            button_id: HardwareButton::Right,
            category: None,
            shown: false,
        }
    }

    fn update(
        &mut self,
        msg: BindingDialogMsg,
        _components: &(),
        _sender: Sender<BindingDialogMsg>,
        parent_sender: Sender<AppMsg>,
    ) {
        match msg {
            BindingDialogMsg::Show(button_id) => {
                self.button_id = button_id;
                self.category = None; // XXX no transition?
                self.shown = true;
            }
            BindingDialogMsg::Hide => {
                self.shown = false;
            }
            BindingDialogMsg::SelectCategory(category) => {
                self.category = category;
            }
            BindingDialogMsg::Selected(entry) => {
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
impl Widgets<BindingDialogModel, super::AppModel> for BindingDialogWidgets {
    view! {
        gtk4::Dialog {
            set_transient_for: parent!(Some(&parent_widgets.main_window)),
            set_default_size: args!(300, 300),
            set_modal: true,
            set_hide_on_close: true,
            set_visible: watch!(model.shown),
            set_titlebar = Some(&gtk4::HeaderBar) {
                pack_start = &gtk4::Button {
                    add_css_class: "flat",
                    set_visible: watch!(model.category.is_some()),
                    set_icon_name: "go-previous-symbolic",
                    connect_clicked(sender) => move |_| {
                        send!(sender, BindingDialogMsg::SelectCategory(None));
                    }
                },
            },
            set_child = Some(&gtk4::ScrolledWindow) {
                set_hscrollbar_policy: gtk4::PolicyType::Never,
                set_child: stack = Some(&gtk4::Stack) {
                    set_hexpand: true,
                    set_margin_start: 12,
                    set_margin_end: 12,
                    set_margin_top: 12,
                    set_margin_bottom: 12,
                    set_vhomogeneous: false,
                    set_transition_type: gtk4::StackTransitionType::SlideLeftRight,
                    add_child: category_list_box = &gtk4::ListBox {
                        set_hexpand: true,
                        add_css_class: "frame",
                        set_header_func: util::header_func,
                        connect_row_activated(sender) => move |_, row| {
                            let category = categories[row.index() as usize];
                            send!(sender, BindingDialogMsg::SelectCategory(Some(category)));
                        },
                    },
                    add_child: binding_list_box = &gtk4::ListBox {
                        set_hexpand: true,
                        add_css_class: "frame",
                        set_header_func: util::header_func,
                        set_filter_func(category, rows) => move |row| {
                            let row_category = rows[row.index() as usize].0;
                            ptr::eq(row_category, category.get())
                        },
                        connect_row_activated(rows) => move |_, row| {
                            let entry = rows[row.index() as usize].1;
                            send!(sender, BindingDialogMsg::Selected(entry));

                        },
                    },
                }
            }
        }
    }

    additional_fields! {
        category: Rc<Cell<&'static Category>>,
    }

    fn pre_init() {
        let mut categories = Vec::new();
        let mut rows = Vec::new();
        for category in &*BINDINGS {
            categories.push(category);
            for entry in &category.entries {
                rows.push((category, entry));
            }
        }
        let rows = Rc::new(rows);

        let category = Rc::new(Cell::new(&BINDINGS[0]));
    }

    fn post_init() {
        for category in &*BINDINGS {
            let mut rows = HashMap::<gtk4::ListBoxRow, &'static Entry>::new();

            view! {
                category_row = gtk4::ListBoxRow {
                    set_selectable: false,
                    set_child: hbox = Some(&gtk4::Box) {
                        set_margin_top: 6,
                        set_margin_bottom: 6,
                        set_margin_start: 6,
                        set_margin_end: 6,
                        set_spacing: 12,
                        set_orientation: gtk4::Orientation::Horizontal,
                        append = &gtk4::Label {
                            set_label: category.label, // TODO Translate?
                        },
                        append = &gtk4::Image {
                            set_hexpand: true,
                            set_halign: gtk4::Align::End,
                            set_icon_name: Some("go-next-symbolic"),
                        }
                    }
                }
            }
            category_list_box.append(&category_row);

            for entry in &category.entries {
                view! {
                    row = gtk4::ListBoxRow {
                        set_selectable: false,
                        set_child: hbox = Some(&gtk4::Box) {
                            set_margin_top: 6,
                            set_margin_bottom: 6,
                            set_margin_start: 6,
                            set_margin_end: 6,
                            set_spacing: 12,
                            set_orientation: gtk4::Orientation::Horizontal,
                            append = &gtk4::Label {
                                set_label: entry.label, // TODO Translate?
                            }
                        }
                    }
                }
                if let Some(keybind) = entry.keybind {
                    view! {
                        keybind_label = gtk4::Label {
                            set_label: keybind,
                            set_hexpand: true,
                            set_halign: gtk4::Align::End,
                        }
                    }
                    hbox.append(&keybind_label);
                }
                binding_list_box.append(&row);
                rows.insert(row, entry);
            }
        }
    }

    fn post_view() {
        if let Some(category) = model.category.as_ref() {
            self.stack.set_visible_child(&self.binding_list_box);
            if !ptr::eq(self.category.get(), *category) {
                self.category.set(*category);
                self.binding_list_box.invalidate_filter();
            }
        } else {
            self.stack.set_visible_child(&self.category_list_box);
        }
    }
}
