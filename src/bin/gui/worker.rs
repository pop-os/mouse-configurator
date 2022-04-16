use gtk4::{pango, prelude::*};
use relm4::{
    send, AppUpdate, ComponentUpdate, Model, RelmApp, RelmComponent, Sender, WidgetPlus, Widgets,
};

pub enum WorkerMsg {}

#[derive(Default)]
pub struct WorkerModel;

impl Model for WorkerModel {
    type Msg = WorkerMsg;
    type Widgets = ();
    type Components = ();
}

impl ComponentUpdate<super::AppModel> for WorkerModel {
    fn init_model(_parent_model: &super::AppModel) -> Self {
        WorkerModel::default()
    }

    fn update(
        &mut self,
        msg: WorkerMsg,
        _components: &(),
        _sender: Sender<WorkerMsg>,
        _parent_sender: Sender<super::AppMsg>,
    ) {
        match msg {}
    }
}
