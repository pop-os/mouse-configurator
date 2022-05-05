use gtk4::{glib, prelude::*, subclass::prelude::*};

use crate::HardwareButton;

pub const IMAGE_WIDTH: i32 = 512;
const IMAGE_RATIO: f64 = 347. / 474.; // Height/width
pub static BUTTONS: &[(f64, f64, bool, Option<HardwareButton>)] = &[
    // Middle click
    (0.9, 0.05, true, Some(HardwareButton::Middle)),
    // Left and right click (swapped in left handed mode)
    (0.085, 0.185, false, None),
    (0.998, 0.185, true, Some(HardwareButton::Right)),
    // Scroll buttons
    (0.1, 0.279, false, Some(HardwareButton::ScrollLeft)),
    (0.985, 0.279, true, Some(HardwareButton::ScrollRight)),
    // Side buttons
    (0.0, 0.51, false, Some(HardwareButton::LeftTop)),
    (0.0, 0.597, false, Some(HardwareButton::LeftCenter)),
    (0.0, 0.68, false, Some(HardwareButton::LeftBottom)),
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
