use crate::value::etype::registry::{EStructId, EStructRegistry};
use crate::value::{EValue, EVector2};
use egui::{Color32, DragValue, Ui};
use rust_i18n::t;
use ustr::{UstrMap, UstrSet};

pub fn draw_number(ui: &mut Ui, name: &str, value: &mut f32) {
    ui.horizontal(|ui| {
        ui.label(name);
        ui.add(DragValue::new(value));
    });
}

pub fn draw_vec2(ui: &mut Ui, name: &str, value: &mut EVector2) {
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

pub fn draw_string(ui: &mut Ui, name: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.label(name);
        ui.text_edit_multiline(value);
    });
}

pub fn draw_boolean(ui: &mut Ui, name: &str, value: &mut bool) {
    ui.checkbox(value, name);
}

fn error(ui: &mut Ui, label: impl Into<String>, text: impl Into<String>) {
    ui.horizontal(|ui| {
        ui.label(label.into());
        ui.colored_label(Color32::RED, text.into());
    });
}

pub fn draw_struct(
    ui: &mut Ui,
    label: &str,
    registry: &EStructRegistry,
    ident: &EStructId,
    fields: &mut UstrMap<EValue>,
) {
    ui.vertical(|ui| match registry.structs().get(ident.raw()) {
        None => error(ui, label, t!("editor.unknown_struct", ident = ident)),
        Some(data) => {
            egui::CollapsingHeader::new(label).show(ui, |ui| {
                let mut extra_fields: UstrSet = fields.keys().copied().collect();

                for f in &data.fields {
                    let value = fields
                        .entry(f.name)
                        .or_insert_with(|| f.ty.default_value(registry));

                    extra_fields.remove(&f.name);
                    draw_evalue(value, ui, f.name.as_str(), registry);
                }

                if !extra_fields.is_empty() {
                    ui.colored_label(Color32::RED, t!("editor.extra_fields"));
                }

                for (field_name, value) in fields {
                    if !extra_fields.contains(field_name) {
                        continue;
                    }
                    draw_evalue(value, ui, field_name.as_str(), registry);
                }
            });
        }
    });
}

pub fn draw_evalue(value: &mut EValue, ui: &mut Ui, label: &str, registry: &EStructRegistry) {
    match value {
        EValue::Boolean { value } => draw_boolean(ui, label, value),
        EValue::Scalar { value } => draw_number(ui, label, value),
        EValue::String { value } => draw_string(ui, label, value),
        EValue::Vec2 { value } => draw_vec2(ui, label, value),
        EValue::Struct { fields, ident } => draw_struct(ui, label, registry, ident, fields),
    }
}
