use crate::value::etype::registry::{EStructRegistry, EStructId};
use crate::value::EValue;
use crate::EditorGraphState;
use egui_node_graph::DataTypeTrait;
use rust_i18n::t;
use std::borrow::Cow;
use std::hash::Hash;
use ustr::Ustr;

pub mod registry;
pub mod serialization;

/// `DataType`s are what defines the possible range of connections when
/// attaching two ports together. The graph UI will make sure to not allow
/// attaching incompatible datatypes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EDataType {
    Boolean,
    Scalar,
    Vec2,
    String,
    Struct { ident: EStructId },
}

impl EDataType {
    pub fn default_value(&self, reg: &EStructRegistry) -> EValue {
        match self {
            EDataType::Boolean => EValue::Boolean { value: false },
            EDataType::Scalar => EValue::Scalar { value: 0.0 },
            EDataType::Vec2 => EValue::Vec2 {
                value: Default::default(),
            },
            EDataType::String => EValue::String {
                value: Default::default(),
            },
            EDataType::Struct { ident } => EValue::Struct {
                ident: *ident,
                fields: reg.default_fields(*ident).unwrap_or_default(),
            },
        }
    }
}

// A trait for the data types, to tell the library how to display them
impl DataTypeTrait<EditorGraphState> for EDataType {
    fn data_type_color(&self, _user_state: &mut EditorGraphState) -> egui::Color32 {
        match self {
            EDataType::Boolean => egui::Color32::from_rgb(211, 109, 25),
            EDataType::Scalar => egui::Color32::from_rgb(38, 109, 211),
            EDataType::Vec2 => egui::Color32::from_rgb(238, 207, 109),
            EDataType::String => egui::Color32::from_rgb(109, 207, 109),
            EDataType::Struct { .. } => egui::Color32::from_rgb(255, 255, 255),
        }
    }

    fn name(&self) -> Cow<'_, str> {
        match self {
            EDataType::Boolean => Cow::Owned(t!("boolean")),
            EDataType::Scalar => Cow::Owned(t!("scalar")),
            EDataType::Vec2 => Cow::Owned(t!("vec2")),
            EDataType::String => Cow::Owned(t!("string")),
            EDataType::Struct { ident, .. } => Cow::Owned(t!(&format!("struct.{ident}"))),
        }
    }
}
