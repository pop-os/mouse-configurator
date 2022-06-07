use gtk4::prelude::*;
use relm4::view;
use std::path::PathBuf;

use crate::util;

pub fn show_about_dialog(main_window: &gtk4::ApplicationWindow) {
    gtk4::AboutDialog::builder()
        .transient_for(main_window)
        .modal(true)
        .version(env!("CARGO_PKG_VERSION"))
        .logo_icon_name("input-mouse-symbolic") // TODO
        .copyright("Copyright 2022 Hewlett-Packard Development Company, L.P.")
        .license_type(gtk4::License::MitX11)
        .build()
        .show()
}

fn device_to_model(device: &str) -> &str {
    if device == "Brain" {
        "HP 930 series Creator Wireless Mouse"
    } else {
        device
    }
}

pub fn show_info_dialog(
    main_window: &gtk4::ApplicationWindow,
    device: &str,
    serial: &str,
    firmware_version: Option<(u16, u16, u16)>,
) {
    view! {
        dialog = gtk4::Dialog {
            set_transient_for: Some(main_window),
            set_modal: true,
            set_title: Some("About This Mouse"),
            set_child = Some(&gtk4::ListBox) {
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_top: 12,
                set_margin_bottom: 12,
                add_css_class: "frame",
                set_header_func: util::header_func,
                append = &gtk4::ListBoxRow {
                    set_margin_start: 6,
                    set_margin_end: 6,
                    set_margin_top: 6,
                    set_margin_bottom: 6,
                    set_selectable: false,
                    set_activatable: false,
                    set_child = Some(&gtk4::Box) {
                        set_spacing: 12,
                        set_orientation: gtk4::Orientation::Horizontal,
                        append = &gtk4::Label {
                            set_label: "Model"
                        },
                        append = &gtk4::Label {
                            set_label: device_to_model(device),
                            set_hexpand: true,
                            set_halign: gtk4::Align::End,
                        }
                    }
                },
                append = &gtk4::ListBoxRow {
                    set_visible: firmware_version.is_some(),
                    set_margin_start: 6,
                    set_margin_end: 6,
                    set_margin_top: 6,
                    set_margin_bottom: 6,
                    set_selectable: false,
                    set_activatable: false,
                    set_child = Some(&gtk4::Box) {
                        set_spacing: 12,
                        set_orientation: gtk4::Orientation::Horizontal,
                        append = &gtk4::Label {
                            set_label: "Firmware Version"
                        },
                        append = &gtk4::Label {
                            set_label: &firmware_version.map_or_else(String::new, |(a, b, c)| format!("{}.{}.{}", a, b, c)),
                            set_hexpand: true,
                            set_halign: gtk4::Align::End,
                        }
                    }
                },
                append = &gtk4::ListBoxRow {
                    set_margin_start: 6,
                    set_margin_end: 6,
                    set_margin_top: 6,
                    set_margin_bottom: 6,
                    set_selectable: false,
                    set_activatable: false,
                    set_child = Some(&gtk4::Box) {
                        set_spacing: 12,
                        set_orientation: gtk4::Orientation::Horizontal,
                        append = &gtk4::Label {
                            set_label: "Unique ID"
                        },
                        append = &gtk4::Label {
                            set_label: serial,
                            set_hexpand: true,
                            set_halign: gtk4::Align::End,
                        }
                    }
                }
            }
        }
    }
    dialog.show();
}

pub fn show_prompt_dialog(
    main_window: &gtk4::ApplicationWindow,
    text: &str,
    cb: impl Fn() + 'static,
) {
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(main_window)
        .modal(true)
        .message_type(gtk4::MessageType::Question)
        .buttons(gtk4::ButtonsType::OkCancel)
        .text(text)
        .build();
    dialog.connect_response(move |dialog, response| {
        if response == gtk4::ResponseType::Ok {
            cb();
        }
        dialog.close();
    });
    dialog.show();
}

pub fn show_error_dialog(main_window: &gtk4::ApplicationWindow, text: &str) {
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(main_window)
        .modal(true)
        .message_type(gtk4::MessageType::Error)
        .buttons(gtk4::ButtonsType::Ok)
        .text(text)
        .build();
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.show();
}

fn show_file_dialog(
    main_window: &gtk4::ApplicationWindow,
    cb: impl Fn(PathBuf) + 'static,
    export: bool,
) {
    let (title, accept_label, action) = if export {
        (
            "Export Configuration",
            "_Export",
            gtk4::FileChooserAction::Save,
        )
    } else {
        (
            "Import Configuration",
            "_Import",
            gtk4::FileChooserAction::Open,
        )
    };

    let dialog = gtk4::FileChooserDialog::new(
        Some(title),
        Some(main_window),
        action,
        &[
            ("_Cancel", gtk4::ResponseType::Cancel),
            (accept_label, gtk4::ResponseType::Ok),
        ],
    );

    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("json"));
    filter.add_pattern("*.json");
    dialog.add_filter(&filter);

    dialog
        .titlebar()
        .unwrap()
        .downcast_ref::<gtk4::HeaderBar>()
        .unwrap()
        .set_show_title_buttons(false);
    dialog.set_modal(true);
    dialog.set_margin_start(6);
    if export {
        dialog.set_current_name("Untitled.json");
    }

    dialog.connect_response(move |dialog, response| {
        if response == gtk4::ResponseType::Ok {
            if let Some(choice) = dialog.file().and_then(|x| x.path()) {
                cb(choice);
            }
        }
        dialog.close();
    });

    dialog.show();
}

pub fn show_import_dialog(main_window: &gtk4::ApplicationWindow, cb: impl Fn(PathBuf) + 'static) {
    show_file_dialog(main_window, cb, false);
}

pub fn show_export_dialog(main_window: &gtk4::ApplicationWindow, cb: impl Fn(PathBuf) + 'static) {
    show_file_dialog(main_window, cb, true);
}
