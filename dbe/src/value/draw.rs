use egui::{DragValue, Ui};
use nalgebra::Vector2;

pub fn draw_f32(ui: &mut Ui, name: &str, value: &mut f32) {
    ui.horizontal(|ui| {
        ui.label(name);
        ui.add(DragValue::new(value));
    });
}

pub fn draw_vec2f32(ui: &mut Ui, name: &str, value: &mut Vector2<f32>) {
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
