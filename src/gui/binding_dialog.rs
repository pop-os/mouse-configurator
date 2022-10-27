use gtk4::{gdk, glib, pango, prelude::*};
use relm4::{send, view, ComponentUpdate, Model, Sender, Widgets};
use std::{cell::Cell, collections::HashMap, ptr, rc::Rc, time::Duration};

use crate::{
    bindings::{Category, Entry, HardwareButton, BINDINGS},
    keycode,
    profile::Binding,
    util, AppMsg,
};

pub enum Page {
    CategoryList,
    Category(&'static Category),
    Custom,
}

pub enum BindingDialogMsg {
    Show(HardwareButton),
    #[allow(unused)]
    Hide,
    SetPage(Page),
    Selected(&'static Entry),
    SetCustomBinding(Option<(i8, i8)>),
    SaveCustom,
}

pub struct BindingDialogModel {
    button_id: HardwareButton,
    page: Page,
    shown: bool,
    custom_binding: Option<(i8, i8)>,
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
            page: Page::CategoryList,
            shown: false,
            custom_binding: None,
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
                self.page = Page::CategoryList;
                self.shown = true;
            }
            BindingDialogMsg::Hide => {
                self.shown = false;
            }
            BindingDialogMsg::SetPage(page) => {
                self.page = page;
                self.custom_binding = None;
            }
            BindingDialogMsg::Selected(entry) => {
                send!(
                    parent_sender,
                    AppMsg::SetBinding(self.button_id, Binding::Preset(entry.id))
                );
                self.shown = false;
            }
            BindingDialogMsg::SetCustomBinding(binding) => {
                self.custom_binding = binding;
            }
            BindingDialogMsg::SaveCustom => {
                if let Some((mods, key)) = self.custom_binding {
                    send!(
                        parent_sender,
                        AppMsg::SetBinding(self.button_id, Binding::Custom(mods, key))
                    );
                    self.shown = false;
                }
            }
        }
    }
}

#[relm4::widget(pub)]
impl Widgets<BindingDialogModel, super::AppModel> for BindingDialogWidgets {
    view! {
        dialog = gtk4::Dialog {
            set_transient_for: parent!(Some(&parent_widgets.main_window)),
            set_default_size: args!(300, 300),
            set_modal: true,
            set_hide_on_close: true,
            set_visible: watch!(model.shown),
            set_title: Some("Set Binding"),
            set_titlebar = Some(&gtk4::HeaderBar) {
                pack_start = &gtk4::Button {
                    add_css_class: "flat",
                    set_visible: watch!(!matches!(model.page, Page::CategoryList)),
                    set_icon_name: "go-previous-symbolic",
                    connect_clicked(sender) => move |_| {
                        send!(sender, BindingDialogMsg::SetPage(Page::CategoryList));
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
                        set_valign: gtk4::Align::Start,
                        set_hexpand: true,
                        add_css_class: "frame",
                        set_header_func: util::header_func,
                        connect_row_activated(sender) => move |_, row| {
                            if row.index() as usize == categories.len() {
                                send!(sender, BindingDialogMsg::SetPage(Page::Custom));
                            } else {
                                let category = categories[row.index() as usize];
                                send!(sender, BindingDialogMsg::SetPage(Page::Category(category)));
                            }
                        },
                    },
                    add_child: binding_vbox = &gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 6,
                        append = &gtk4::Label {
                            set_label: watch! {
                                if let Page::Category(category) = &model.page {
                                    &category.label
                                    } else {
                                        ""
                                    }
                            }, // XXX translate
                            set_attributes = Some(&pango::AttrList) {
                                insert: pango::AttrInt::new_weight(pango::Weight::Bold)
                            },
                        },
                        append: binding_list_box = &gtk4::ListBox {
                            set_hexpand: true,
                            add_css_class: "frame",
                            set_header_func: util::header_func,
                            set_filter_func(category, rows) => move |row| {
                                let row_category = rows[row.index() as usize].0;
                                ptr::eq(row_category, category.get())
                            },
                            connect_row_activated(rows, sender) => move |_, row| {
                                let entry = rows[row.index() as usize].1;
                                send!(sender, BindingDialogMsg::Selected(entry));

                            },
                        },
                    },
                    add_child: custom_binding_stack = &gtk4::Stack {
                        add_child: custom_binding_box = &gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_focusable: true,
                            append = &gtk4::Label {
                                set_label: "Press the key combination to set new shortcut"
                            },
                            add_controller = &gtk4::EventControllerKey {
                                connect_key_released(sender, shortcut_label) => move |_, _keyval, keycode, state| {
                                    println!("keyval: {:?}", _keyval);

                                    // Only set if recongized

                                    let accelerator = keycode::keycode_accelerator(keycode, state);
                                    shortcut_label.set_accelerator(&accelerator.as_deref().unwrap_or(""));

                                    if let Some(keycode) = keycode::gdk_to_mouse_keycode(keycode) {
                                        let mods = keycode::modifier_to_mask(state);
                                        send!(sender, BindingDialogMsg::SetCustomBinding(Some((mods, keycode))));
                                    }
                                }
                            },
                        },
                        add_child: custom_binding_set_box = &gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            append = &gtk4::Label {
                                set_label: "New Custom Shortcut"
                            },
                            append: shortcut_label = &gtk4::ShortcutLabel {
                            },
                            append = &gtk4::Button {
                                set_label: "Save",
                                connect_clicked(sender) => move |_| {
                                    send!(sender, BindingDialogMsg::SaveCustom)
                                }
                            }
                        }
                    },
                }
            }
        }
    }

    additional_fields! {
        category: Rc<Cell<&'static Category>>,
        inhibit_source: Option<glib::SourceId>,
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
        let inhibit_source = None;
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

        view! {
            custom_row = gtk4::ListBoxRow {
                set_selectable: false,
                set_child: hbox = Some(&gtk4::Box) {
                    set_margin_top: 6,
                    set_margin_bottom: 6,
                    set_margin_start: 6,
                    set_margin_end: 6,
                    set_spacing: 12,
                    set_orientation: gtk4::Orientation::Horizontal,
                    append = &gtk4::Label {
                        set_label: "Custom Shortcut", // TODO Translate?
                    },
                    append = &gtk4::Image {
                        set_hexpand: true,
                        set_halign: gtk4::Align::End,
                        set_icon_name: Some("go-next-symbolic"),
                    }
                }
            }
        }
        category_list_box.append(&custom_row);

        // Avoid transition on reopening
        dialog.connect_visible_notify(
            glib::clone!(@strong stack, @strong category_list_box => move |dialog| {
                if !dialog.is_visible() {
                    stack.set_transition_type(gtk4::StackTransitionType::None);
                    stack.set_visible_child(&category_list_box);
                    stack.set_transition_type(gtk4::StackTransitionType::SlideLeftRight);
                }
            }),
        );
    }

    fn post_view() {
        match &model.page {
            Page::CategoryList => {
                self.stack.set_visible_child(&self.category_list_box);
            }
            Page::Category(category) => {
                self.stack.set_visible_child(&self.binding_vbox);
                if !ptr::eq(self.category.get(), *category) {
                    self.category.set(*category);
                    self.binding_list_box.invalidate_filter();
                }
            }
            Page::Custom => {
                self.stack.set_visible_child(&self.custom_binding_stack);
                if model.custom_binding.is_none() {
                    self.custom_binding_stack
                        .set_visible_child(&self.custom_binding_box);
                    println!("{}", self.custom_binding_box.grab_focus());
                } else {
                    self.custom_binding_stack
                        .set_visible_child(&self.custom_binding_set_box);
                }
            }
        }

        // Inhibit system shorcuts if on `Custom` page
        let surface = self.dialog.surface().downcast::<gdk::Toplevel>().unwrap();
        let should_inhibit = matches!(model.page, Page::Custom) && model.custom_binding.is_none();
        if !should_inhibit {
            if let Some(inhibit_source) = self.inhibit_source.take() {
                inhibit_source.remove();
                surface.restore_system_shortcuts();
            }
        } else if self.inhibit_source.is_none() {
            self.inhibit_source = Some(glib::timeout_add_local(
                Duration::from_millis(100),
                move || {
                    surface.inhibit_system_shortcuts(None::<&gdk::Event>);
                    Continue(true)
                },
            ));
        }
    }
}
