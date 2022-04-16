use gtk4::{pango, prelude::*};
use relm4::{
    send, AppUpdate, ComponentUpdate, Model, RelmApp, RelmComponent, Sender, WidgetPlus, Widgets,
};

pub enum DialogMsg {
    Show(u8), // XXX button
    Hide,
}

#[derive(Default)]
pub struct DialogModel {
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
        _parent_sender: Sender<super::AppMsg>,
    ) {
        match msg {
            DialogMsg::Show(button) => {
                self.shown = true;
            }
            DialogMsg::Hide => {
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
            set_modal: true,
            set_visible: watch!(model.shown),
            set_child = Some(&gtk4::ScrolledWindow) {
                set_hscrollbar_policy: gtk4::PolicyType::Never,
                set_child = Some(&gtk4::Box) {
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
}
