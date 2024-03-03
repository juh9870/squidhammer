use std::fmt::{Debug, Display};
use std::ops::RangeInclusive;

use anyhow::{anyhow, bail, Context};
use camino::Utf8PathBuf;
use dyn_clone::DynClone;
use egui::collapsing_header::CollapsingState;
use egui::{
    Align, Direction, DragValue, InnerResponse, RichText, Slider, TextEdit, Ui, Widget, WidgetText,
};
use itertools::Itertools;
use ordered_float::Float;
use rust_i18n::t;
use rustc_hash::FxHashMap;
use tracing::{error, trace};
use ustr::{Ustr, UstrMap};

use egui_node_graph::NodeId;

use utils::mem_temp;

use crate::graph::event::EditorGraphResponse;

use crate::value::etype::registry::eenum::{
    EEnumData, EEnumVariant, EEnumVariantId, EEnumVariantWithId, EnumPattern,
};
use crate::value::etype::registry::eitem::EItemType;

use crate::value::etype::registry::{ETypeId, ETypesRegistry, EValueId};
use crate::value::etype::{EDataType, ETypeConst};
use crate::value::{ENumber, EValue};

/// Upper bound size guarantees of different editors
///
/// Editor may take up less space than what is specified by this enum, but
/// promise to not take any more than specified
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum EditorSize {
    /// Editors with this size promise to take up no space in UI
    None,
    /// Editors with this size promise to reasonably fit as a part of a single
    /// line, along with other content
    Inline,
    /// Editors with this size may occupy up to a whole line
    SingleLine,
    /// Editors with this size may occupy more than one line
    Block,
}

impl EditorSize {
    pub fn is_inline(&self) -> bool {
        matches!(self, EditorSize::Inline)
    }

    pub fn is_single_line(&self) -> bool {
        matches!(self, EditorSize::SingleLine)
    }
    pub fn is_block(&self) -> bool {
        matches!(self, EditorSize::Block)
    }
}

pub trait EFieldEditor: Debug + Send + DynClone {
    fn inputs(&self) -> Vec<(String, EItemType)> {
        vec![]
    }
    fn output(&self) -> EDataType;
    fn size(&self) -> EditorSize;
    fn draw(
        &self,
        ui: &mut Ui,
        registry: &ETypesRegistry,
        path: &FieldPath,
        field_name: &str,
        value: &mut EValue,
        editors: &FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
        responses: &mut Vec<EditorGraphResponse>,
    );
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
            "string:multiline".to_string(),
            Box::new(StringEditor { multiline: true }),
        ),
        ("boolean".to_string(), Box::new(BooleanEditor)),
        // Enums
        (
            "enum".to_string(),
            Box::new(EnumEditorConstructor::from(EnumEditorType::Auto)),
        ),
        (
            "enum:toggle".to_string(),
            Box::new(EnumEditorConstructor::from(EnumEditorType::Toggle)),
        ),
        (
            "enum:full".to_string(),
            Box::new(EnumEditorConstructor::from(EnumEditorType::Full)),
        ),
        ("const".to_string(), Box::new(ConstEditorConstructor)),
        ("id".to_string(), Box::new(ObjectIdEditorConstructor)),
        // other
        ("rgb".to_string(), Box::new(RgbEditorConstructor::rgb())),
        ("rgba".to_string(), Box::new(RgbEditorConstructor::rgba())),
    ];
    v.into_iter()
}

// region utilities

fn inline_error(ui: &mut Ui, err: impl Into<anyhow::Error>) {
    ui.label(RichText::new(err.into().to_string()).color(ui.style().visuals.error_fg_color));
}

fn labeled_field<T>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    content: impl FnOnce(&mut Ui) -> T,
) -> InnerResponse<T> {
    ui.horizontal(|ui| {
        let text = label.into();
        if !text.is_empty() {
            ui.label(text);
        }
        content(ui)
    })
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

macro_rules! unsupported {
    ($ui:expr, $label:expr, $value:expr, $editor:expr) => {
        // tracing::warn!(value=?$value, editor=?$editor, "Unsupported value for editor");
        labeled_error(
            $ui,
            $label,
            anyhow!("{}", t!("dbe.editor.unsupported_value")),
        );
        return
    };
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

// endregion

// region error

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

    fn size(&self) -> EditorSize {
        EditorSize::Inline
    }

    fn draw(
        &self,
        ui: &mut Ui,
        _registry: &ETypesRegistry,
        _path: &FieldPath,
        field_name: &str,
        _value: &mut EValue,
        _editors: &FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
        _responses: &mut Vec<EditorGraphResponse>,
    ) {
        labeled_error(ui, field_name, anyhow!("{}", self.error))
    }
}

// endregion

// region number

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

    fn size(&self) -> EditorSize {
        EditorSize::Inline
    }

    fn draw(
        &self,
        ui: &mut Ui,
        _registry: &ETypesRegistry,
        _path: &FieldPath,
        field_name: &str,
        value: &mut EValue,
        _editors: &FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
        _responses: &mut Vec<EditorGraphResponse>,
    ) {
        let Ok(value) = value.try_as_number_mut() else {
            unsupported!(ui, field_name, value, self);
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
        Ok(Box::new(NumberEditor {
            range,
            logarithmic: ty.logarithmic,
            slider: self.slider,
        }))
    }
}

// endregion

// region string

#[derive(Debug, Clone)]
struct StringEditor {
    multiline: bool,
}

impl EFieldEditor for StringEditor {
    fn output(&self) -> EDataType {
        EDataType::String
    }

    fn size(&self) -> EditorSize {
        if self.multiline {
            EditorSize::Block
        } else {
            EditorSize::Inline
        }
    }

    fn draw(
        &self,
        ui: &mut Ui,
        _registry: &ETypesRegistry,
        _path: &FieldPath,
        field_name: &str,
        value: &mut EValue,
        _editors: &FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
        _responses: &mut Vec<EditorGraphResponse>,
    ) {
        let Ok(value) = value.try_as_string_mut() else {
            unsupported!(ui, field_name, value, self);
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

// endregion

// region boolean

#[derive(Debug, Clone)]
struct BooleanEditor;

impl EFieldEditor for BooleanEditor {
    fn output(&self) -> EDataType {
        EDataType::Boolean
    }

    fn size(&self) -> EditorSize {
        EditorSize::Inline
    }

    fn draw(
        &self,
        ui: &mut Ui,
        _registry: &ETypesRegistry,
        _path: &FieldPath,
        field_name: &str,
        value: &mut EValue,
        _editors: &FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
        _responses: &mut Vec<EditorGraphResponse>,
    ) {
        let Ok(value) = value.try_as_boolean_mut() else {
            unsupported!(ui, field_name, value, self);
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

// endregion

// region Rgb

#[derive(Debug, Clone)]
struct RgbEditor {
    with_alpha: bool,
    ident: ETypeId,
}

impl EFieldEditor for RgbEditor {
    fn output(&self) -> EDataType {
        EDataType::Object { ident: self.ident }
    }

    fn size(&self) -> EditorSize {
        EditorSize::Block
    }

    fn draw(
        &self,
        ui: &mut Ui,
        _registry: &ETypesRegistry,
        _path: &FieldPath,
        field_name: &str,
        value: &mut EValue,
        _editors: &FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
        _responses: &mut Vec<EditorGraphResponse>,
    ) {
        let field_names = ["r", "g", "b", if self.with_alpha { "a" } else { "" }];
        let EValue::Struct { fields, .. } = value else {
            unsupported!(ui, field_name, value, self);
        };

        CollapsingState::load_with_default_open(ui.ctx(), ui.id().with(field_name), false)
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

// endregion

// region enum

struct EnumEditorData<'a> {
    registry: &'a ETypesRegistry,
    path: &'a FieldPath,
    field_name: &'a str,
    variant: &'a mut EEnumVariantId,
    value: &'a mut EValue,
    editors: &'a FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
    responses: &'a mut Vec<EditorGraphResponse>,

    content_path: FieldPath,
    skip_draw_body: bool,
    content_editor_size: EditorSize,
    enum_data: &'a EEnumData,
    selected_variant: &'a EEnumVariant,

    new_value: Option<EValue>,
}

impl<'a> EnumEditorData<'a> {
    pub fn init(
        ui: &mut Ui,
        registry: &'a ETypesRegistry,
        path: &'a FieldPath,
        field_name: &'a str,
        variant: &'a mut EEnumVariantId,
        value: &'a mut EValue,
        editors: &'a FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
        responses: &'a mut Vec<EditorGraphResponse>,
    ) -> Option<Self> {
        let Some((enum_data, selected_variant)) = variant.enum_variant(registry) else {
            labeled_error(ui, field_name, anyhow!("Failed to find enum variant"));
            return None;
        };

        let content_path = path.with("content");
        let mut skip_draw_body = false;
        let content_editor_size = if let Some(editor) = editors.get(&content_path.path) {
            editor.size()
        } else {
            let editor = registry.editor_for_or_err(None, &selected_variant.data);
            let size = editor.size();
            responses.push(EditorGraphResponse::ChangeEditor {
                editor,
                path: content_path.clone(),
            });
            // Skip drawing body to avoid issues with a default editor
            skip_draw_body = true;
            size
        };

        Some(Self {
            registry,
            path,
            field_name,
            variant,
            value,
            editors,
            responses,
            content_path,
            skip_draw_body,
            content_editor_size,
            enum_data,
            selected_variant,
            new_value: None,
        })
    }

    fn hide_const_body(&mut self) {
        if matches!(self.selected_variant.pat, EnumPattern::Const(_)) {
            self.skip_draw_body = true;
        }
    }

    fn body_size(&self) -> EditorSize {
        if self.skip_draw_body {
            EditorSize::None
        } else {
            self.content_editor_size
        }
    }

    fn can_be_toggle(&self) -> bool {
        self.enum_data.variants().len() == 2
    }

    fn change_variant(&mut self, new_variant: EEnumVariantId) {
        *self.variant = new_variant;
        match new_variant.variant(self.registry) {
            None => {
                error!(id=?new_variant, path=?self.path, "Failed to obtain enum variant for ID")
            }
            Some(variant) => {
                let editor = self.registry.editor_for_or_err(None, &variant.data);
                self.responses.push(EditorGraphResponse::ChangeEditor {
                    editor,
                    path: self.content_path.clone(),
                });
                self.new_value = Some(variant.default_value(self.registry));
            }
        }
    }

    fn picker(&mut self, ui: &mut Ui) {
        let mut selected = *self.variant;
        egui::ComboBox::from_id_source(self.field_name)
            .selected_text(&self.selected_variant.name)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for (variant, id) in self.enum_data.variants_with_ids() {
                    ui.selectable_value(&mut selected, *id, &variant.name);
                }
            });
        if &selected != self.variant {
            self.change_variant(selected)
        }
    }

    fn toggle_data(&self) -> anyhow::Result<(EEnumVariantWithId, EEnumVariantWithId)> {
        let mut iter = self.enum_data.variants_with_ids();
        let first = iter
            .next()
            .context("Toggle enum editor requires exactly two enum variants, got zero")?;
        let second = iter
            .next()
            .context("Toggle enum editor requires exactly two enum variants, got one")?;
        let count = iter.count();
        if count > 0 {
            bail!(
                "Toggle enum editor requires exactly two enum variants, got {}",
                count + 2
            )
        }
        Ok((first, second))
    }

    fn toggle_editor(&mut self, ui: &mut Ui) {
        self.toggle_editor_custom(ui, |ui, checked, first, second| {
            ui.toggle_value(
                checked,
                if *checked {
                    &first.0.name
                } else {
                    &second.0.name
                },
            );
        })
    }

    fn toggle_editor_custom(
        &mut self,
        ui: &mut Ui,
        checkbox: impl FnOnce(&mut Ui, &mut bool, EEnumVariantWithId, EEnumVariantWithId),
    ) {
        let (first, second) = match self.toggle_data() {
            Ok(data) => data,
            Err(err) => {
                inline_error(ui, err);
                return;
            }
        };

        let checked = self.variant == first.1;

        let mut after_check = checked;
        checkbox(ui, &mut after_check, first, second);
        if after_check != checked {
            self.change_variant(if after_check { *first.1 } else { *second.1 })
        }
    }

    fn body(self, ui: &mut Ui) {
        if !self.skip_draw_body {
            value_widget(
                ui,
                self.value,
                &self.content_path,
                "",
                self.registry,
                self.editors,
                self.responses,
            );
        }
        if let Some(value) = self.new_value {
            *self.value = value
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum EnumEditorType {
    Auto,
    Full,
    Toggle,
}

#[derive(Debug, Clone)]
struct EnumEditor {
    ident: ETypeId,
    ty: EnumEditorType,
}
impl EFieldEditor for EnumEditor {
    fn output(&self) -> EDataType {
        EDataType::Object { ident: self.ident }
    }

    fn size(&self) -> EditorSize {
        EditorSize::Block
    }

    fn draw(
        &self,
        ui: &mut Ui,
        registry: &ETypesRegistry,
        path: &FieldPath,
        field_name: &str,
        value: &mut EValue,
        editors: &FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
        responses: &mut Vec<EditorGraphResponse>,
    ) {
        let EValue::Enum {
            variant,
            data: value,
        } = value
        else {
            unsupported!(ui, field_name, value, self);
        };

        let Some(mut editor) = EnumEditorData::init(
            ui, registry, path, field_name, variant, value, editors, responses,
        ) else {
            return;
        };
        editor.hide_const_body();

        match self.ty {
            EnumEditorType::Toggle | EnumEditorType::Auto if editor.can_be_toggle() => {
                if editor.body_size().is_block() {
                    CollapsingState::load_with_default_open(
                        ui.ctx(),
                        ui.id().with(field_name),
                        true,
                    )
                    .show_header(ui, |ui| {
                        labeled_field(ui, field_name, |ui| editor.toggle_editor(ui))
                    })
                    .body(|ui| editor.body(ui));
                } else {
                    let dir = if editor.body_size() <= EditorSize::Inline {
                        Direction::LeftToRight
                    } else {
                        Direction::TopDown
                    };

                    ui.with_layout(
                        egui::Layout::from_main_dir_and_cross_align(dir, Align::Min),
                        |ui| {
                            labeled_field(ui, field_name, |ui| editor.toggle_editor(ui));
                            editor.body(ui);
                        },
                    );
                }
            }
            _ => {
                if editor.body_size().is_block() {
                    CollapsingState::load_with_default_open(
                        ui.ctx(),
                        ui.id().with(field_name),
                        true,
                    )
                    .show_header(ui, |ui| {
                        labeled_field(ui, field_name, |ui| editor.picker(ui))
                    })
                    .body(|ui| editor.body(ui));
                } else {
                    labeled_field(ui, field_name, |ui| editor.picker(ui));
                    editor.body(ui);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct EnumEditorConstructor {
    ty: EnumEditorType,
}

impl From<EnumEditorType> for EnumEditorConstructor {
    fn from(value: EnumEditorType) -> Self {
        EnumEditorConstructor { ty: value }
    }
}

impl EFieldEditorConstructor for EnumEditorConstructor {
    fn make_editor(&self, item: &EItemType) -> anyhow::Result<Box<dyn EFieldEditor>> {
        let EItemType::Enum(e) = item else {
            bail!("Unsupported item")
        };

        Ok(Box::new(EnumEditor {
            ident: e.id,
            ty: self.ty,
        }))
    }
}

// endregion

// region const

#[derive(Debug, Clone)]
struct ConstEditor {
    item: ETypeConst,
}

impl EFieldEditor for ConstEditor {
    fn output(&self) -> EDataType {
        EDataType::Const { value: self.item }
    }

    fn size(&self) -> EditorSize {
        EditorSize::Inline
    }

    fn draw(
        &self,
        ui: &mut Ui,
        _registry: &ETypesRegistry,
        _path: &FieldPath,
        field_name: &str,
        value: &mut EValue,
        _editors: &FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
        _responses: &mut Vec<EditorGraphResponse>,
    ) {
        let const_value = self.item.default_value();
        if value != &const_value {
            labeled_error(ui, field_name, anyhow!("{}", t!("dbe.editor.bad_const")))
        }

        labeled_field(ui, field_name, |ui| ui.label(value.to_string()));
    }
}

#[derive(Debug, Clone)]
struct ConstEditorConstructor;

impl EFieldEditorConstructor for ConstEditorConstructor {
    fn make_editor(&self, item: &EItemType) -> anyhow::Result<Box<dyn EFieldEditor>> {
        let EItemType::Const(c) = item else {
            bail!("Unsupported item")
        };

        Ok(Box::new(ConstEditor { item: c.value }))
    }
}

// endregion

// region object ID

#[derive(Debug, Clone)]
struct ObjectIdEditor {
    ty: ETypeId,
}

impl EFieldEditor for ObjectIdEditor {
    fn output(&self) -> EDataType {
        EDataType::Id { ty: self.ty }
    }

    fn size(&self) -> EditorSize {
        EditorSize::Inline
    }

    fn draw(
        &self,
        ui: &mut Ui,
        _registry: &ETypesRegistry,
        _path: &FieldPath,
        field_name: &str,
        value: &mut EValue,
        _editors: &FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
        _responses: &mut Vec<EditorGraphResponse>,
    ) {
        let EValue::Id { value, .. } = value else {
            unsupported!(ui, field_name, value, self);
        };

        let text_store = ui.id().with("editor storage");

        let mut fresh = false;

        let (mut text, mut err): (String, Option<String>) = match mem_temp!(ui, text_store) {
            Some(data) => data,
            None => {
                fresh = true;
                (value.map(|e| e.to_string()).unwrap_or_default(), None)
            }
        };

        let res = labeled_field(ui, field_name, |ui| {
            let color = if err.is_none() {
                ui.style().visuals.text_color()
            } else {
                ui.style().visuals.error_fg_color
            };
            if err.is_some() {
                ui.label(RichText::new("⚠").color(color));
            } else {
                ui.label("✅");
            }
            let res = TextEdit::singleline(&mut text)
                .hint_text("mod_id:item_id")
                .text_color(color)
                .ui(ui);
            if res.changed() || fresh {
                text.retain(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '/' | ':'));
                match EValueId::parse(&text) {
                    Ok(id) => {
                        *value = Some(id);
                        err = None
                    }
                    Err(error) => err = Some(error.to_string()),
                }
            }
        });

        if let Some(err) = &err {
            res.response.on_hover_text(err);
        }

        mem_temp!(ui, text_store, (text, err))
    }
}

#[derive(Debug, Copy, Clone)]
struct ObjectIdEditorConstructor;
impl EFieldEditorConstructor for ObjectIdEditorConstructor {
    fn make_editor(&self, item: &EItemType) -> anyhow::Result<Box<dyn EFieldEditor>> {
        let EItemType::ObjectId(id) = item else {
            bail!("Unsupported item")
        };

        Ok(Box::new(ObjectIdEditor { ty: id.ty }))
    }
}

// endregion

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FieldPath {
    pub node: NodeId,
    pub path: Utf8PathBuf,
}

impl FieldPath {
    pub fn new(node: NodeId) -> Self {
        Self {
            node,
            path: Utf8PathBuf::from("/"),
        }
    }

    pub fn with(&self, name: &str) -> Self {
        Self {
            node: self.node,
            path: self.path.join(name),
        }
    }
}

pub fn value_widget(
    ui: &mut Ui,
    value: &mut EValue,
    field_path: &FieldPath,
    label: &str,
    registry: &ETypesRegistry,
    editors: &FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
    responses: &mut Vec<EditorGraphResponse>,
) {
    ui.push_id(field_path, |ui| {
        match editors.get(&field_path.path) {
            None => {
                let editor = registry.editor_for_or_err(None, &EItemType::default_item_for(value));
                trace!(?field_path, label, ?editor, "New editor is requested");
                // We use clone because drawing an editor might in turn request
                // another editor, and we don't want to override that
                responses.push(EditorGraphResponse::ChangeEditor {
                    editor: editor.clone(),
                    path: field_path.clone(),
                });
                editor.draw(ui, registry, field_path, label, value, editors, responses);
            }
            Some(editor) => {
                editor.draw(ui, registry, field_path, label, value, editors, responses);
            }
        };
    });
}
