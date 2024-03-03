use crate::etype::econst::ETypeConst;
use crate::registry::ETypesRegistry;
use crate::value::id::{EListId, EMapId, ETypeId};
use crate::value::EValue;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use strum::EnumIs;

pub mod econst;
pub mod eenum;
pub mod eitem;
pub mod estruct;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, EnumIs)]
pub enum EDataType {
    /// Primitive boolean type
    Boolean,
    /// Primitive numeric type
    Number,
    /// Primitive string type
    String,
    /// Object ID type
    Id {
        ty: ETypeId,
    },
    /// Object reference type
    Ref {
        ty: ETypeId,
    },
    /// Inline object, enum, or list type
    Object {
        ident: ETypeId,
    },
    /// Primitive constant type
    Const {
        value: ETypeConst,
    },
    List {
        id: EListId,
    },
    Map {
        id: EMapId,
    },
}

impl EDataType {
    pub fn default_value(&self, reg: &ETypesRegistry) -> EValue {
        match self {
            EDataType::Boolean => EValue::Boolean { value: false },
            EDataType::Number => EValue::Number { value: 0.0.into() },
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
            EDataType::List { .. } => todo!(),
            EDataType::Map { .. } => todo!(),
        }
    }

    pub const fn null() -> EDataType {
        EDataType::Const {
            value: ETypeConst::Null,
        }
    }

    pub fn name(&self) -> Cow<'_, str> {
        match self {
            EDataType::Boolean => "boolean".into(),
            EDataType::Number => "number".into(),
            EDataType::String => "string".into(),
            EDataType::Id { ty } => format!("Id<{}>", ty).into(),
            EDataType::Ref { ty } => format!("Ref<{}>", ty).into(),
            EDataType::Object { ident } => ident.to_string().into(),
            EDataType::Const { value } => value.to_string().into(),
            EDataType::List { id: ty } => ty.to_string().into(),
            EDataType::Map { id: ty } => ty.to_string().into(),
        }
    }
}
