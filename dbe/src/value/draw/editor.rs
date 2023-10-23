use std::fmt::Debug;
use std::ops::RangeInclusive;

use anyhow::{anyhow, bail};
use dyn_clone::DynClone;
use egui::{DragValue, RichText, Slider, Ui, WidgetText};
use ordered_float::Float;
use rust_i18n::t;

use crate::value::etype::registry::eitem::EItemType;
use crate::value::etype::registry::ETypesRegistry;
use crate::value::etype::EDataType;
use crate::value::{ENumber, EValue};

pub trait EFieldEditor: Debug + Send + DynClone {
    fn inputs(&self) -> Vec<(String, EItemType)> {
        vec![]
    }
    fn output(&self) -> EDataType;
    fn draw(&self, ui: &mut Ui, registry: &ETypesRegistry, field_name: &str, value: &mut EValue);
}
dyn_clone::clone_trait_object!(EFieldEditor);

pub trait EFieldEditorConstructor: Debug {
    fn make_editor(&self, item: EItemType) -> anyhow::Result<Box<dyn EFieldEditor>>;
}

pub fn default_editors() -> impl Iterator<Item = (String, Box<dyn EFieldEditorConstructor>)> {
    let v: Vec<(String, Box<dyn EFieldEditorConstructor>)> = vec![
        (
            "number".to_string(),
            Box::new(NumberEditorConstructor { slider: false }),
        ),
        (
            "slider".to_string(),
            Box::new(NumberEditorConstructor { slider: true }),
        ),
        (
            "string".to_string(),
            Box::new(StringEditor { multiline: false }),
        ),
        (
            "multiline".to_string(),
            Box::new(StringEditor { multiline: false }),
        ),
        ("boolean".to_string(), Box::new(BooleanEditor)),
    ];
    v.into_iter()
}

fn labeled_field(ui: &mut Ui, label: impl Into<WidgetText>, content: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.label(label);
        content(ui)
    });
}

fn labeled_error(ui: &mut Ui, label: impl Into<WidgetText>, err: impl Into<anyhow::Error>) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label(RichText::new(err.into().to_string()).color(ui.style().visuals.error_fg_color))
    });
}

fn unsupported(ui: &mut Ui, label: impl Into<WidgetText>) {
    labeled_error(ui, label, anyhow!("{}", t!("dbe.editor.unsupported_value")));
}

#[derive(Debug, Clone)]
pub struct EFieldEditorError {
    error: String,
    output: EDataType,
}

impl EFieldEditorError {
    pub fn new(error: String, output: EDataType) -> Self {
        Self { error, output }
    }
}

impl EFieldEditor for EFieldEditorError {
    fn output(&self) -> EDataType {
        self.output
    }

    fn draw(&self, ui: &mut Ui, _registry: &ETypesRegistry, field_name: &str, _value: &mut EValue) {
        labeled_error(ui, field_name, anyhow!("{}", self.error))
    }
}

#[derive(Debug, Clone)]
struct NumberEditor {
    range: RangeInclusive<ENumber>,
    logarithmic: Option<bool>,
    slider: bool,
}

impl EFieldEditor for NumberEditor {
    fn output(&self) -> EDataType {
        EDataType::Number
    }

    fn draw(&self, ui: &mut Ui, _registry: &ETypesRegistry, field_name: &str, value: &mut EValue) {
        let Ok(value) = value.try_as_number_mut() else {
            unsupported(ui, field_name);
            return;
        };
        labeled_field(ui, field_name, |ui| {
            if self.slider {
                let log = self
                    .logarithmic
                    .unwrap_or_else(|| self.range.end() - self.range.start() >= 1e6);
                ui.add(Slider::new(value, self.range.clone()).logarithmic(log));
            } else {
                ui.add(DragValue::new(value).clamp_range(self.range.clone()));
            }
        });
    }
}

#[derive(Debug)]
struct NumberEditorConstructor {
    slider: bool,
}

impl EFieldEditorConstructor for NumberEditorConstructor {
    fn make_editor(&self, item: EItemType) -> anyhow::Result<Box<dyn EFieldEditor>> {
        let EItemType::Number(ty) = item else {
            bail!("Unsupported item")
        };

        let range = ty.min.unwrap_or(ENumber::min_value())..=ty.max.unwrap_or(ENumber::max_value());
        return Ok(Box::new(NumberEditor {
            range,
            logarithmic: ty.logarithmic,
            slider: self.slider,
        }));
    }
}

#[derive(Debug, Clone)]
struct StringEditor {
    multiline: bool,
}

impl EFieldEditor for StringEditor {
    fn output(&self) -> EDataType {
        EDataType::String
    }
    fn draw(&self, ui: &mut Ui, _registry: &ETypesRegistry, field_name: &str, value: &mut EValue) {
        let Ok(value) = value.try_as_string_mut() else {
            unsupported(ui, field_name);
            return;
        };
        labeled_field(ui, field_name, |ui| {
            if self.multiline {
                ui.text_edit_multiline(value);
            } else {
                ui.text_edit_singleline(value);
            }
        });
    }
}

impl EFieldEditorConstructor for StringEditor {
    fn make_editor(&self, item: EItemType) -> anyhow::Result<Box<dyn EFieldEditor>> {
        let EItemType::String(_) = item else {
            bail!("Unsupported item")
        };

        Ok(Box::new(self.clone()))
    }
}

#[derive(Debug, Clone)]
struct BooleanEditor;

impl EFieldEditor for BooleanEditor {
    fn output(&self) -> EDataType {
        EDataType::Boolean
    }

    fn draw(&self, ui: &mut Ui, _registry: &ETypesRegistry, field_name: &str, value: &mut EValue) {
        let Ok(value) = value.try_as_boolean_mut() else {
            unsupported(ui, field_name);
            return;
        };
        ui.checkbox(value, field_name);
    }
}

impl EFieldEditorConstructor for BooleanEditor {
    fn make_editor(&self, item: EItemType) -> anyhow::Result<Box<dyn EFieldEditor>> {
        let EItemType::Boolean(_) = item else {
            bail!("Unsupported item")
        };
        Ok(Box::new(self.clone()))
    }
}

// #[derive(Debug, knuffel::DecodeScalar, Default, Copy, Clone)]
// pub enum ColorEditorType {
//     #[default]
//     Default,
// }
//
// impl StructFieldEditor<Rgba, EItemColor> for ColorEditorType {
//     fn edit(
//         &self,
//         ui: &mut Ui,
//         _registry: &ETypesRegistry,
//         value: &mut Rgba,
//         field: &EStructField,
//         ty: &EItemColor,
//     ) {
//         labeled_field(ui, field, |ui| {
//             ui.horizontal(|ui| {
//                 let mut color = value.to_rgba_unmultiplied();
//                 let format = ty.format;
//                 for channel in format.channels() {
//                     match channel {
//                         ColorChannel::None => {}
//                         ColorChannel::Red => {
//                             ui.label("R");
//                             ui.add(DragValue::new(&mut color[0]).clamp_range(0..=1).speed(0.01));
//                         }
//                         ColorChannel::Green => {
//                             ui.label("G");
//                             ui.add(DragValue::new(&mut color[1]).clamp_range(0..=1).speed(0.01));
//                         }
//                         ColorChannel::Blue => {
//                             ui.label("B");
//                             ui.add(DragValue::new(&mut color[2]).clamp_range(0..=1).speed(0.01));
//                         }
//                         ColorChannel::Alpha => {
//                             ui.label("A");
//                             ui.add(DragValue::new(&mut color[3]).clamp_range(0..=1).speed(0.01));
//                         }
//                     }
//                 }
//                 if format.with_alpha() {
//                     ui.color_edit_button_rgba_unmultiplied(&mut color);
//                 } else {
//                     let mut rgb = [color[0], color[1], color[2]];
//                     ui.color_edit_button_rgb(&mut rgb);
//                     color = [rgb[0], rgb[1], rgb[2], color[3]];
//                 }
//                 *value = Rgba::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
//             });
//         });
//     }
// }
