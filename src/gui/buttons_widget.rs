use gtk4::{glib, prelude::*, subclass::prelude::*};

use crate::HardwareButton;

const SVG_WIDTH: f64 = 474.;
const SVG_HEIGHT: f64 = 347.;

pub const IMAGE_WIDTH: i32 = 512;
const IMAGE_RATIO: f64 = SVG_HEIGHT / SVG_WIDTH;
pub static BUTTONS: &[(f64, f64, bool, Option<HardwareButton>)] = &[
    // Middle click
    (426.6, 17.35, true, Some(HardwareButton::Middle)),
    // Left and right click (swapped in left handed mode)
    (40.29, 64.195, false, None),
    (473.052, 64.195, true, Some(HardwareButton::Right)),
    // Scroll buttons
    (47.4, 96.813, false, Some(HardwareButton::ScrollLeft)),
    (466.89, 96.813, true, Some(HardwareButton::ScrollRight)),
    // Side buttons
    (0.0, 176.97, false, Some(HardwareButton::LeftTop)),
    (0.0, 207.159, false, Some(HardwareButton::LeftCenter)),
    (0.0, 235.96, false, Some(HardwareButton::LeftBottom)),
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
            y * h / SVG_HEIGHT,
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
            x * w / SVG_WIDTH,
            0,
        ));
    }
}
