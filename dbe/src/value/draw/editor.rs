use crate::value::etype::registry::estruct::{
    EStructFieldBoolean, EStructFieldColor, EStructFieldScalar, EStructFieldString,
    EStructFieldType,
};
use crate::value::etype::registry::ETypesRegistry;
use crate::value::ENumber;
use egui::{Color32, DragValue, Rgba, RichText, Slider, Ui};
use ordered_float::Float;
use utils::color_format::{ColorChannel, ColorFormat};

fn labeled_field(ui: &mut Ui, field: &impl EStructFieldType, content: impl FnOnce(&mut Ui)) {
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
    fn edit(&self, ui: &mut Ui, registry: &ETypesRegistry, value: &mut Data, field: &Field);
}

impl StructFieldEditor<ENumber, EStructFieldScalar> for ScalarEditorType {
    fn edit(
        &self,
        ui: &mut Ui,
        registry: &ETypesRegistry,
        value: &mut ENumber,
        field: &EStructFieldScalar,
    ) {
        labeled_field(ui, field, |ui| {
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
    fn edit(
        &self,
        ui: &mut Ui,
        registry: &ETypesRegistry,
        value: &mut String,
        field: &EStructFieldString,
    ) {
        match self {
            StringEditorType::SingleLine => {
                labeled_field(ui, field, |ui| {
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
pub enum ColorEditorType {
    #[default]
    Default,
}

impl StructFieldEditor<Rgba, EStructFieldColor> for ColorEditorType {
    fn edit(
        &self,
        ui: &mut Ui,
        registry: &ETypesRegistry,
        value: &mut Rgba,
        field: &EStructFieldColor,
    ) {
        labeled_field(ui, field, |ui| {
            ui.horizontal(|ui| {
                let mut color = value.to_rgba_unmultiplied();
                let format = field.format();
                for channel in format.channels() {
                    match channel {
                        ColorChannel::None => {}
                        ColorChannel::Red => {
                            ui.label("R");
                            ui.add(DragValue::new(&mut color[0]).clamp_range(0..=1).speed(0.01));
                        }
                        ColorChannel::Green => {
                            ui.label("G");
                            ui.add(DragValue::new(&mut color[1]).clamp_range(0..=1).speed(0.01));
                        }
                        ColorChannel::Blue => {
                            ui.label("B");
                            ui.add(DragValue::new(&mut color[2]).clamp_range(0..=1).speed(0.01));
                        }
                        ColorChannel::Alpha => {
                            ui.label("A");
                            ui.add(DragValue::new(&mut color[3]).clamp_range(0..=1).speed(0.01));
                        }
                    }
                }
                if format.with_alpha() {
                    ui.color_edit_button_rgba_unmultiplied(&mut color);
                } else {
                    let mut rgb = [color[0], color[1], color[2]];
                    ui.color_edit_button_rgb(&mut rgb);
                    color = [rgb[0], rgb[1], rgb[2], color[3]];
                }
                *value = Rgba::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
            });
        });
    }
}

#[derive(Debug, knuffel::DecodeScalar, Default, Copy, Clone)]
pub enum BooleanEditorType {
    #[default]
    Checkbox,
}

impl StructFieldEditor<bool, EStructFieldBoolean> for BooleanEditorType {
    fn edit(
        &self,
        ui: &mut Ui,
        registry: &ETypesRegistry,
        value: &mut bool,
        field: &EStructFieldBoolean,
    ) {
        ui.checkbox(value, field.name().as_str());
    }
}
