use gtk4::prelude::*;

pub fn header_func(row: &gtk4::ListBoxRow, before: Option<&gtk4::ListBoxRow>) {
    if before.is_none() {
        row.set_header(None::<&gtk4::Widget>)
    } else if row.header().is_none() {
        row.set_header(Some(&gtk4::Separator::new(gtk4::Orientation::Horizontal)));
    }
}
