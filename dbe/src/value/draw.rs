use crate::value::etype::registry::{EObjectType, ETypesRegistry, ETypetId};
use crate::value::{ENumber, EValue, EVector2, JsonValue};
use egui::{Color32, DragValue, Ui};
use rust_i18n::t;
use serde_json::{Number, Value};
use ustr::{UstrMap, UstrSet};

pub fn draw_number(ui: &mut Ui, name: &str, value: &mut ENumber) {
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

pub fn draw_json(ui: &mut Ui, name: &str, value: &mut JsonValue) {
    match value {
        Value::Null => {
            ui.label("null");
        }
        Value::Bool(bool) => {
            draw_boolean(ui, name, bool);
        }
        Value::Number(num) => {
            let mut f_num = num.as_f64().expect("Should always convert to double");
            ui.horizontal(|ui| {
                ui.label(name);
                ui.add(DragValue::new(&mut f_num));
            });
            if let Some(f_num) = Number::from_f64(f_num) {
                *num = f_num;
            }
        }
        Value::String(str) => {
            draw_string(ui, name, str);
        }
        Value::Array(_) => {
            ui.label("[Array]");
        }
        Value::Object(_) => {
            ui.label("[Object]");
        }
    };
}

pub fn draw_unknown(ui: &mut Ui, name: &str, value: &mut JsonValue) {
    egui::CollapsingHeader::new(t!("editor.unknown_json"))
        .show(ui, |ui| draw_json(ui, name, value));
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
    registry: &ETypesRegistry,
    ident: &ETypetId,
    fields: &mut UstrMap<EValue>,
) {
    ui.vertical(|ui| match registry.get_object(ident) {
        None => error(ui, label, t!("editor.unknown_struct", ident = ident)),
        Some(EObjectType::Struct(data)) => {
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
        Some(_) => error(ui, label, t!("editor.not_a_struct", ident = ident)),
    });
}

pub fn draw_evalue(value: &mut EValue, ui: &mut Ui, label: &str, registry: &ETypesRegistry) {
    match value {
        EValue::Unknown { value: inner } => draw_unknown(ui, label, inner),
        EValue::Boolean { value } => draw_boolean(ui, label, value),
        EValue::Scalar { value } => draw_number(ui, label, value),
        EValue::String { value } => draw_string(ui, label, value),
        EValue::Vec2 { value } => draw_vec2(ui, label, value),
        EValue::Struct { fields, ident } => draw_struct(ui, label, registry, ident, fields),
        EValue::Enum { .. } => todo!(),
        EValue::Id { .. } => todo!(),
        EValue::Ref { .. } => todo!(),
    }
}
