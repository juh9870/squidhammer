use std::fmt::{Debug, Display};
use std::ops::RangeInclusive;

use anyhow::{anyhow, bail, Context};
use dyn_clone::DynClone;
use egui::collapsing_header::CollapsingState;
use egui::{DragValue, RichText, Slider, Ui, WidgetText};
use itertools::Itertools;
use ordered_float::Float;
use rust_i18n::t;
use ustr::{Ustr, UstrMap};

use crate::value::etype::registry::eitem::EItemType;
use crate::value::etype::registry::{ETypeId, ETypesRegistry};
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
    fn make_editor(&self, item: &EItemType) -> anyhow::Result<Box<dyn EFieldEditor>>;
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
            Box::new(StringEditor { multiline: true }),
        ),
        ("boolean".to_string(), Box::new(BooleanEditor)),
        ("rgb".to_string(), Box::new(RgbEditorConstructor::rgb())),
        ("rgba".to_string(), Box::new(RgbEditorConstructor::rgba())),
    ];
    v.into_iter()
}

fn inline_error(ui: &mut Ui, err: impl Into<anyhow::Error>) {
    ui.label(RichText::new(err.into().to_string()).color(ui.style().visuals.error_fg_color));
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
        inline_error(ui, err);
    });
}

fn unsupported(ui: &mut Ui, label: impl Into<WidgetText>) {
    labeled_error(ui, label, anyhow!("{}", t!("dbe.editor.unsupported_value")));
}

// fn ensure_fields(
//     ui: &mut Ui,
//     label: impl Into<WidgetText>,
//     value: &mut EValue,
//     required: impl IntoIterator<Item = impl AsRef<str> + Display>,
//     editor: impl FnOnce(&mut Ui, &mut UstrMap<EValue>),
// ) -> bool {
//     let EValue::Struct { fields, .. } = value else {
//         unsupported(ui, label);
//         return false;
//     };
//     let missing = required
//         .into_iter()
//         .filter(|e| !e.as_ref().is_empty() && !fields.contains_key(&e.as_ref().into()))
//         .join(", ");
//     if missing.is_empty() {
//         editor(ui, fields);
//         true
//     } else {
//         labeled_error(
//             ui,
//             label,
//             anyhow!("Required fields are missing: {}", missing),
//         );
//         false
//     }
// }

fn ensure_field<'a, T: TryFrom<&'a mut EValue, Error = E>, E: Into<anyhow::Error>>(
    ui: &mut Ui,
    fields: &'a mut UstrMap<EValue>,
    field_name: impl AsRef<str> + Display,
    editor: impl FnOnce(&mut Ui, T),
) -> bool {
    let name = field_name.as_ref();
    let value = fields.get_mut(&name.into());

    let Some(val) = value else {
        labeled_error(ui, name, anyhow!("Field is missing"));
        return false;
    };

    let val: Result<T, T::Error> = val.try_into();
    match val {
        Err(err) => {
            labeled_error(ui, name, err);
            false
        }
        Ok(data) => {
            editor(ui, data);
            true
        }
    }
}

fn get_values<'a, T: TryFrom<&'a EValue, Error = E>, E: Into<anyhow::Error>, const N: usize>(
    fields: &'a UstrMap<EValue>,
    names: [&str; N],
) -> anyhow::Result<[T; N]> {
    let vec: Vec<T> = names
        .into_iter()
        .map(|name| {
            fields
                .get(&name.into())
                .with_context(|| format!("Field {name} is missing"))
                .and_then(|value| T::try_from(value).map_err(|err| err.into()))
        })
        .try_collect()?;

    Ok(vec
        .try_into()
        .map_err(|_| unreachable!("Length did not change"))
        .unwrap())
}

fn set_values<'a>(
    fields: &mut UstrMap<EValue>,
    entries: impl IntoIterator<Item = (&'a str, impl Into<EValue>)>,
) {
    let entries = entries.into_iter().map(|(k, v)| (Ustr::from(k), v.into()));
    fields.extend(entries);
}

trait EditorResultExt {
    type Data;
    fn then_draw<Res>(
        self,
        ui: &mut Ui,
        draw: impl FnOnce(&mut Ui, Self::Data) -> Res,
    ) -> Option<Res>;
}

impl<T, Err: Into<anyhow::Error>> EditorResultExt for Result<T, Err> {
    type Data = T;

    fn then_draw<Res>(
        self,
        ui: &mut Ui,
        draw: impl FnOnce(&mut Ui, Self::Data) -> Res,
    ) -> Option<Res> {
        match self {
            Err(err) => {
                inline_error(ui, err);
                None
            }
            Ok(data) => Some(draw(ui, data)),
        }
    }
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
    fn make_editor(&self, item: &EItemType) -> anyhow::Result<Box<dyn EFieldEditor>> {
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
    fn make_editor(&self, item: &EItemType) -> anyhow::Result<Box<dyn EFieldEditor>> {
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
        labeled_field(ui, field_name, |ui| {
            ui.toggle_value(value, if *value { "⏹ True" } else { "☐ False" });
        });
    }
}

impl EFieldEditorConstructor for BooleanEditor {
    fn make_editor(&self, item: &EItemType) -> anyhow::Result<Box<dyn EFieldEditor>> {
        let EItemType::Boolean(_) = item else {
            bail!("Unsupported item")
        };
        Ok(Box::new(self.clone()))
    }
}

#[derive(Debug, Clone)]
struct RgbEditor {
    with_alpha: bool,
    ident: ETypeId,
}

impl EFieldEditor for RgbEditor {
    fn output(&self) -> EDataType {
        EDataType::Object { ident: self.ident }
    }

    fn draw(&self, ui: &mut Ui, _registry: &ETypesRegistry, field_name: &str, value: &mut EValue) {
        let field_names = ["r", "g", "b", if self.with_alpha { "a" } else { "" }];
        let EValue::Struct { fields, .. } = value else {
            unsupported(ui, field_name);
            return;
        };

        CollapsingState::load_with_default_open(ui.ctx(), egui::Id::new(field_name), false)
            .show_header(ui, |ui| {
                labeled_field(ui, field_name, |ui| {
                    if self.with_alpha {
                        get_values::<ENumber, _, 4>(fields, ["r", "g", "b", "a"]).then_draw(
                            ui,
                            |ui, mut value| {
                                ui.color_edit_button_rgba_unmultiplied(&mut value);
                                set_values(
                                    fields,
                                    [
                                        ("r", value[0]),
                                        ("g", value[1]),
                                        ("b", value[2]),
                                        ("a", value[3]),
                                    ],
                                )
                            },
                        );
                    } else {
                        get_values::<ENumber, _, 3>(fields, ["r", "g", "b"]).then_draw(
                            ui,
                            |ui, mut value| {
                                ui.color_edit_button_rgb(&mut value);
                                set_values(
                                    fields,
                                    [("r", value[0]), ("g", value[1]), ("b", value[2])],
                                );
                            },
                        );
                    }
                });
            })
            .body(|ui| {
                ui.vertical(|ui| {
                    for name in field_names {
                        ensure_field(ui, fields, name, |ui, value: &mut ENumber| {
                            labeled_field(ui, name, |ui| {
                                ui.add(DragValue::new(value).clamp_range(0..=1).speed(0.01));
                            });
                        });
                    }
                })
            });
    }
}

#[derive(Debug, Clone)]
struct RgbEditorConstructor {
    with_alpha: bool,
}

impl RgbEditorConstructor {
    pub fn rgba() -> Self {
        Self { with_alpha: true }
    }
    pub fn rgb() -> Self {
        Self { with_alpha: false }
    }
}

impl EFieldEditorConstructor for RgbEditorConstructor {
    fn make_editor(&self, item: &EItemType) -> anyhow::Result<Box<dyn EFieldEditor>> {
        let EItemType::Struct(s) = item else {
            bail!("Unsupported item")
        };

        Ok(Box::new(RgbEditor {
            with_alpha: self.with_alpha,
            ident: s.id,
        }))
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
