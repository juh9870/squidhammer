use crate::value::etype::registry::estruct::{
    EStructFieldBoolean, EStructFieldScalar, EStructFieldString, EStructFieldType,
};
use crate::value::ENumber;
use egui::{DragValue, Slider, Ui};
use ordered_float::Float;

fn labeled(ui: &mut Ui, field: &impl EStructFieldType, content: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.label(field.name().as_str());
        content(ui)
    });
}

#[derive(Debug, knuffel::DecodeScalar, Default, Copy, Clone, Eq, PartialEq)]
pub enum ScalarType {
    #[default]
    Decimal,
    Int,
}

#[derive(Debug, knuffel::DecodeScalar, Default, Copy, Clone)]
pub enum ScalarEditorType {
    #[default]
    Default,
    Slider,
}

pub trait StructFieldEditor<Data, Field> {
    fn edit(&self, ui: &mut Ui, value: &mut Data, field: &Field);
}

impl StructFieldEditor<ENumber, EStructFieldScalar> for ScalarEditorType {
    fn edit(&self, ui: &mut Ui, value: &mut ENumber, field: &EStructFieldScalar) {
        labeled(ui, field, |ui| {
            let range = field.min().unwrap_or(ENumber::min_value())
                ..=field.max().unwrap_or(ENumber::max_value());
            match self {
                ScalarEditorType::Default => {
                    ui.add(DragValue::new(value).clamp_range(range));
                }
                ScalarEditorType::Slider => {
                    let log = field
                        .logarithmic()
                        .unwrap_or_else(|| range.end() - range.start() >= 1e6);
                    ui.add(Slider::new(value, range).logarithmic(log));
                }
            }
        });
        if field.scalar_ty() == ScalarType::Int {
            *value = value.round();
            if let Some(min) = field.min() {
                if *value < min {
                    *value = min.ceil()
                }
            }
            if let Some(max) = field.max() {
                if *value > max {
                    *value = max.floor()
                }
            }
        }
    }
}

#[derive(Debug, knuffel::DecodeScalar, Default, Copy, Clone)]
pub enum StringEditorType {
    #[default]
    SingleLine,
    Multiline,
}

impl StructFieldEditor<String, EStructFieldString> for StringEditorType {
    fn edit(&self, ui: &mut Ui, value: &mut String, field: &EStructFieldString) {
        match self {
            StringEditorType::SingleLine => {
                labeled(ui, field, |ui| {
                    ui.text_edit_singleline(value);
                });
            }
            StringEditorType::Multiline => {
                ui.vertical(|ui| {
                    ui.label(field.name().as_str());
                    ui.text_edit_multiline(value);
                });
            }
        }
    }
}

#[derive(Debug, knuffel::DecodeScalar, Default, Copy, Clone)]
pub enum BooleanEditorType {
    #[default]
    Checkbox,
}

impl StructFieldEditor<bool, EStructFieldBoolean> for BooleanEditorType {
    fn edit(&self, ui: &mut Ui, value: &mut bool, field: &EStructFieldBoolean) {
        ui.checkbox(value, field.name().as_str());
    }
}
