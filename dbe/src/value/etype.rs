use crate::value::etype::registry::{ETypesRegistry, ETypetId};
use crate::value::{ENumber, EValue};
use crate::EditorGraphState;
use egui_node_graph::DataTypeTrait;
use rust_i18n::t;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use ustr::Ustr;

pub mod registry;

/// `DataType`s are what defines the possible range of connections when
/// attaching two ports together. The graph UI will make sure to not allow
/// attaching incompatible datatypes.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EDataType {
    Boolean,
    Scalar,
    Vec2,
    String,
    Object { ident: ETypetId },
    Const { value: ETypeConst },
}

impl EDataType {
    pub fn default_value(&self, reg: &ETypesRegistry) -> EValue {
        match self {
            EDataType::Boolean => EValue::Boolean { value: false },
            EDataType::Scalar => EValue::Scalar { value: 0.0 },
            EDataType::Vec2 => EValue::Vec2 {
                value: Default::default(),
            },
            EDataType::String => EValue::String {
                value: Default::default(),
            },
            EDataType::Object { ident } => reg.default_value(ident),
            EDataType::Const { value } => value.default_value(),
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
            EDataType::Object { .. } => egui::Color32::from_rgb(255, 255, 255),
            EDataType::Const { .. } => todo!(),
        }
    }

    fn name(&self) -> Cow<'_, str> {
        match self {
            EDataType::Boolean => Cow::Owned(t!("boolean")),
            EDataType::Scalar => Cow::Owned(t!("scalar")),
            EDataType::Vec2 => Cow::Owned(t!("vec2")),
            EDataType::String => Cow::Owned(t!("string")),
            EDataType::Object { ident } => Cow::Owned(t!(ident.raw())),
            EDataType::Const { value } => Cow::Owned(value.default_value().to_string()),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ETypeConst {
    Boolean(bool),
    Scalar(ordered_float::OrderedFloat<ENumber>),
    String(Ustr),
}

impl ETypeConst {
    pub fn default_value(&self) -> EValue {
        match self {
            ETypeConst::Boolean(value) => (*value).into(),
            ETypeConst::Scalar(value) => value.0.into(),
            ETypeConst::String(value) => value.to_string().into(),
        }
    }
}

impl Display for ETypeConst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ETypeConst::Boolean(value) => write!(f, "{value}"),
            ETypeConst::Scalar(value) => write!(f, "{value}"),
            ETypeConst::String(value) => write!(f, "'{value}'"),
        }
    }
}
