use egui::{Response, Ui};

pub fn toggle_button(ui: &mut Ui, value: &mut bool) -> Response {
    ui.toggle_value(value, if *value { "⏹ True" } else { "☐ False" })
}

pub fn toggle_button_label(ui: &mut Ui, label: &str, value: &mut bool) -> Response {
    ui.horizontal(|ui| {
        ui.label(label) | ui.toggle_value(value, if *value { "⏹ True" } else { "☐ False" })
    })
    .inner
}
