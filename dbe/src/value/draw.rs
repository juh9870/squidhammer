use crate::value::EVector2;
use egui::{DragValue, Ui};

pub fn draw_f32(ui: &mut Ui, name: &str, value: &mut f32) {
    ui.horizontal(|ui| {
        ui.label(name);
        ui.add(DragValue::new(value));
    });
}

pub fn draw_vec2f32(ui: &mut Ui, name: &str, value: &mut EVector2) {
    ui.horizontal(|ui| {
        ui.label(name);
        ui.horizontal(|ui| {
            ui.label("x");
            ui.add(DragValue::new(&mut value.x));
            ui.label("y");
            ui.add(DragValue::new(&mut value.y));
        });
    });
}
