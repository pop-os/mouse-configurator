use gtk4::prelude::*;
use relm4::{send, ComponentUpdate, Model, Sender, Widgets};

use crate::AppMsg;

pub enum SwapButtonDialogMsg {
    Show(bool),
    SetLeftHanded(bool),
    Close,
}

#[derive(Default)]
pub struct SwapButtonDialogModel {
    shown: bool,
    left_handed: bool,
}

impl Model for SwapButtonDialogModel {
    type Msg = SwapButtonDialogMsg;
    type Widgets = SwapButtonDialogWidgets;
    type Components = ();
}

impl ComponentUpdate<super::AppModel> for SwapButtonDialogModel {
    fn init_model(_parent_model: &super::AppModel) -> Self {
        SwapButtonDialogModel::default()
    }

    fn update(
        &mut self,
        msg: SwapButtonDialogMsg,
        _components: &(),
        _sender: Sender<SwapButtonDialogMsg>,
        parent_sender: Sender<AppMsg>,
    ) {
        match msg {
            SwapButtonDialogMsg::Show(left_handed) => {
                self.left_handed = left_handed;
                self.shown = true;
            }
            SwapButtonDialogMsg::SetLeftHanded(left_handed) => {
                self.left_handed = left_handed;
                send!(parent_sender, AppMsg::SetLeftHanded(left_handed));
            }
            SwapButtonDialogMsg::Close => {
                self.shown = false;
            }
        }
    }
}

#[relm4::widget(pub)]
impl Widgets<SwapButtonDialogModel, super::AppModel> for SwapButtonDialogWidgets {
    view! {
        dialog_with_header() -> gtk4::Dialog {
            set_transient_for: parent!(Some(&parent_widgets.main_window)),
            set_modal: true,
            set_hide_on_close: true,
            set_title: Some("Swap Left and Right Mouse Buttons"),
            set_visible: watch!(model.shown),
            connect_response(sender) => move |_, _| {
                send!(sender, SwapButtonDialogMsg::Close)
             },
            set_child: vbox = Some(&gtk4::Box) {
                set_orientation: gtk4::Orientation::Vertical,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_top: 12,
                set_margin_bottom: 12,
                set_spacing: 6,
                append = &gtk4::Label {
                    set_label: "Select the button you use for primary functions such as selecting and dragging."
                },
                append = &gtk4::Box {
                    set_halign: gtk4::Align::Center,
                    set_orientation: gtk4::Orientation::Horizontal,
                    append = &gtk4::ToggleButton {
                        set_active: watch! { !model.left_handed },
                        set_label: "Left",
                        connect_clicked(sender) => move |_| {
                            send!(sender, SwapButtonDialogMsg::SetLeftHanded(false))
                        }
                    },
                    append = &gtk4::ToggleButton {
                        set_active: watch! { model.left_handed },
                        set_label: "Right",
                        connect_clicked(sender) => move |_| {
                            send!(sender, SwapButtonDialogMsg::SetLeftHanded(true))
                        }
                    }
                }
            }
        }
    }
}

fn dialog_with_header() -> gtk4::Dialog {
    gtk4::Dialog::builder().use_header_bar(1).build()
}
