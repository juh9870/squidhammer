use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::hash::Hash;

use ordered_float::OrderedFloat;
use random_color::{Luminosity, RandomColor};
use serde::{Deserialize, Serialize};
use ustr::Ustr;

use egui_node_graph::{DataTypeMatcherMarker, DataTypeTrait};

use crate::value::etype::registry::{ETypeId, ETypesRegistry};
use crate::value::{ENumber, EValue};
use crate::EditorGraphState;

pub mod registry;

/// `DataType`s are what defines the possible range of connections when
/// attaching two ports together. The graph UI will make sure to not allow
/// attaching incompatible datatypes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EDataType {
    /// Primitive boolean type
    Boolean,
    /// Primitive numeric type
    Number,
    /// Primitive string type
    String,
    /// Object ID type
    Id { ty: ETypeId },
    /// Object reference type
    Ref { ty: ETypeId },
    /// Inline object, enum, or list type
    Object { ident: ETypeId },
    /// Primitive constant type
    Const { value: ETypeConst },
}

impl EDataType {
    pub fn default_value(&self, reg: &ETypesRegistry) -> EValue {
        match self {
            EDataType::Boolean => EValue::Boolean { value: false },
            EDataType::Number => EValue::Number { value: 0.0 },
            EDataType::String => EValue::String {
                value: Default::default(),
            },
            EDataType::Object { ident } => reg.default_value(ident),
            EDataType::Const { value } => value.default_value(),
            EDataType::Id { ty } => EValue::Id {
                ty: *ty,
                value: None,
            },
            EDataType::Ref { ty } => EValue::Ref {
                ty: *ty,
                value: None,
            },
        }
    }
}

impl DataTypeMatcherMarker for EDataType {}

// A trait for the data types, to tell the library how to display them
impl DataTypeTrait<EditorGraphState> for EDataType {
    fn data_type_color(&self, _user_state: &mut EditorGraphState) -> egui::Color32 {
        let c = RandomColor::new()
            .seed(self.name().to_string())
            .luminosity(Luminosity::Dark)
            .alpha(1.0)
            .to_rgb_array();
        egui::Color32::from_rgb(c[0], c[1], c[2])
    }

    fn name(&self) -> Cow<'_, str> {
        match self {
            EDataType::Boolean => Cow::Borrowed("boolean"),
            EDataType::Number => Cow::Borrowed("number"),
            EDataType::String => Cow::Borrowed("string"),
            EDataType::Id { ty } => Cow::Owned(format!("Id<{}>", ty)),
            EDataType::Ref { ty } => Cow::Owned(format!("Ref<{}>", ty)),
            EDataType::Object { ident } => Cow::Owned(ident.to_string()),
            EDataType::Const { value } => Cow::Owned(value.to_string()),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ETypeConst {
    String(Ustr),
    Number(OrderedFloat<ENumber>),
    Boolean(bool),
    Null,
}

impl ETypeConst {
    pub fn default_value(&self) -> EValue {
        match self {
            ETypeConst::Boolean(value) => (*value).into(),
            ETypeConst::Number(value) => value.0.into(),
            ETypeConst::String(value) => value.to_string().into(),
            ETypeConst::Null => EValue::Null,
        }
    }
}

impl Display for ETypeConst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ETypeConst::Boolean(value) => write!(f, "{value}"),
            ETypeConst::Number(value) => write!(f, "{value}"),
            ETypeConst::String(value) => write!(f, "'{value}'"),
            ETypeConst::Null => write!(f, "null"),
        }
    }
}
