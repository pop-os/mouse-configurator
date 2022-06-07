use gtk4::{glib, prelude::*, subclass::prelude::*};

use crate::HardwareButton;

const SVG_WIDTH: f64 = 474.;
const SVG_HEIGHT: f64 = 347.;

pub const IMAGE_WIDTH: i32 = 512;
const IMAGE_RATIO: f64 = SVG_HEIGHT / SVG_WIDTH;
pub static BUTTONS: &[(f64, f64, bool, Option<HardwareButton>)] = &[
    // Middle click
    (342., 17., true, Some(HardwareButton::Middle)),
    // Left and right click (swapped in left handed mode)
    (121., 64., false, None),
    (392., 64., true, Some(HardwareButton::Right)),
    // Scroll buttons
    (121., 97., false, Some(HardwareButton::ScrollLeft)),
    (392., 97., true, Some(HardwareButton::ScrollRight)),
    // Side buttons
    (89., 178., false, Some(HardwareButton::LeftTop)),
    (89., 207., false, Some(HardwareButton::LeftCenter)),
    (89., 236., false, Some(HardwareButton::LeftBottom)),
];

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
    pub fn add_button(&self, button: &gtk4::Button, x: f64, y: f64, right: bool) {
        let w = IMAGE_WIDTH as f64;
        let h = w * IMAGE_RATIO;

        let layout_manager: gtk4::ConstraintLayout =
            self.layout_manager().unwrap().downcast().unwrap();
        button.set_parent(self);
        layout_manager.add_constraint(&gtk4::Constraint::new_constant(
            Some(button),
            gtk4::ConstraintAttribute::CenterY,
            gtk4::ConstraintRelation::Eq,
            y * h / SVG_HEIGHT,
            0,
        ));
        let side = if right {
            gtk4::ConstraintAttribute::Left
        } else {
            gtk4::ConstraintAttribute::Right
        };
        layout_manager.add_constraint(&gtk4::Constraint::new_constant(
            Some(button),
            side,
            gtk4::ConstraintRelation::Eq,
            x * w / SVG_WIDTH,
            0,
        ));
    }
}
