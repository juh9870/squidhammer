use crate::value::etype::registry::eitem::{EItemType, EItemTypeTrait};
use crate::value::etype::registry::{EObjectType, ETypeId, ETypesRegistry};
use crate::value::{ENumber, EValue, EVector2, JsonValue};
use egui::{Color32, DragValue, RichText, Ui, WidgetText};
use rust_i18n::t;
use serde_json::{Number, Value};
use ustr::{UstrMap, UstrSet};

pub mod editor;

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

fn error(ui: &mut Ui, label: impl Into<WidgetText>, text: impl Into<RichText>) {
    ui.horizontal(|ui| {
        ui.label(label.into());
        ui.colored_label(Color32::RED, text.into());
    });
}

pub fn draw_struct(
    ui: &mut Ui,
    registry: &ETypesRegistry,
    ident: &ETypeId,
    fields: &mut UstrMap<EValue>,
) {
    ui.vertical(|ui| match registry.get_object(ident) {
        None => error(ui, "struct", t!("editor.unknown_struct", ident = ident)),
        Some(EObjectType::Struct(data)) => {
            let mut extra_fields: UstrSet = fields.keys().copied().collect();

            for f in &data.fields {
                let _value = fields
                    .entry(f.name)
                    .or_insert_with(|| f.ty.default_value(registry));

                extra_fields.remove(&f.name);
                todo!()
                // draw_default_field_editor(ui, registry, value, f);
            }

            if !extra_fields.is_empty() {
                ui.colored_label(Color32::RED, t!("editor.extra_fields"));
            }

            for (field_name, _value) in fields {
                if !extra_fields.contains(field_name) {
                    continue;
                }
                error(ui, field_name.as_str(), t!("editor.unexpected_field"))
                // draw_evalue(value, ui, field_name.as_str(), registry);
            }
        }
        Some(_) => error(ui, "struct", t!("editor.not_a_struct", ident = ident)),
    });
}

fn draw_default_field_editor(
    _ui: &mut Ui,
    _registry: &ETypesRegistry,
    _value: &mut EValue,
    _name: String,
    _f: &EItemType,
) {
    todo!()
    // ui.horizontal(|ui| {
    //     match (value, &f.ty) {
    //         (EValue::Number { value }, EItemType::Number(field)) => {
    //             field.editor.edit(ui, registry, value, f, field)
    //         }
    //         (EValue::String { value }, EItemType::String(field)) => {
    //             field.editor.edit(ui, registry, value, f, field)
    //         }
    //         (EValue::Boolean { value }, EItemType::Boolean(field)) => {
    //             field.editor.edit(ui, registry, value, f, field)
    //         }
    //         (value, EItemType::Const(field)) => {
    //             if &field.value.default_value() != value {
    //                 error(
    //                     ui,
    //                     f.name.as_str(),
    //                     "Value type is incompatible with field type",
    //                 )
    //             }
    //             ui.horizontal(|ui| {
    //                 ui.label(name());
    //                 ui.label(value.to_string())
    //             });
    //         }
    //         (EValue::Struct { .. }, EItemType::Struct(..)) => {
    //             error(ui, name(), "TODO");
    //         }
    //         (EValue::Enum { .. }, EItemType::Enum(_)) => {
    //             error(ui, name(), "TODO");
    //         }
    //         // EStructField::Enum(_) => {}
    //         _ => error(
    //             ui,
    //             name(),
    //             format!("Value type is incompatible with field type"),
    //         ),
    //     }
    // });
}

// pub fn draw_evalue(value: &mut EValue, ui: &mut Ui, label: &str, registry: &ETypesRegistry) {
//     match value {
//         EValue::Unknown { value: inner } => draw_unknown(ui, label, inner),
//         EValue::Boolean { value } => draw_boolean(ui, label, value),
//         EValue::Scalar { value } => draw_number(ui, label, value),
//         EValue::String { value } => draw_string(ui, label, value),
//         EValue::Vec2 { value } => draw_vec2(ui, label, value),
//         EValue::Struct { fields, ident } => draw_struct(ui, label, registry, ident, fields),
//         EValue::Enum { .. } => todo!(),
//         EValue::Id { .. } => todo!(),
//         EValue::Ref { .. } => todo!(),
//     }
// }
