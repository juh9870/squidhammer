use crate::value::etype::registry::{EObjectType, ETypesRegistry, ETypetId};
use crate::value::etype::{EDataType, ETypeConst};
use crate::value::{EValue, JsonValue};
use anyhow::{anyhow, bail, Context};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::{Display, Formatter};
use ustr::Ustr;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub(super) enum EnumPattern {
    StructField(Ustr, ETypeConst),
    Boolean,
    Scalar,
    Vec2,
    String,
    Const(ETypeConst),
}

impl Display for EnumPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EnumPattern::StructField(field, ty) => write!(f, "{{\"{field}\": \"{ty}\"}}"),
            EnumPattern::Boolean => write!(f, "{{boolean}}"),
            EnumPattern::Scalar => write!(f, "{{number}}"),
            EnumPattern::Vec2 => write!(f, "{{vec2}}"),
            EnumPattern::String => write!(f, "{{string}}"),
            EnumPattern::Const(ty) => write!(f, "{{{ty}}}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EEnumVariant {
    pat: EnumPattern,
    data: EDataType,
    name: String,
}

impl EEnumVariant {
    pub fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        self.data.default_value(registry)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn new(name: String, pat: EnumPattern, data: EDataType) -> Self {
        Self { pat, data, name }
    }

    pub(super) fn boolean(name: String) -> EEnumVariant {
        Self {
            data: EDataType::Boolean,
            pat: EnumPattern::Boolean,
            name,
        }
    }

    pub(super) fn scalar(name: String) -> EEnumVariant {
        Self {
            data: EDataType::Scalar,
            pat: EnumPattern::Scalar,
            name,
        }
    }

    // pub(super) fn vec2() -> EEnumVariant {
    //     Self {
    //         data: EDataType::Vec2,
    //         pat: EnumPattern::Vec2,
    //     }
    // }

    pub(super) fn string(name: String) -> EEnumVariant {
        Self {
            data: EDataType::String,
            pat: EnumPattern::String,
            name,
        }
    }

    pub(super) fn econst(name: String, data: ETypeConst) -> EEnumVariant {
        Self {
            data: EDataType::Const { value: data },
            pat: EnumPattern::Const(data),
            name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EEnumData {
    pub ident: ETypetId,
    pub variants: Vec<EEnumVariant>,
}

impl EEnumData {
    pub fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        let default_variant = self.variants.first().expect("Expect enum to not be empty");
        EValue::Enum {
            variant: EEnumVariantId {
                ident: self.ident,
                variant: default_variant.pat,
            },
            data: Box::new(default_variant.default_value(registry)),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct EEnumVariantId {
    ident: ETypetId,
    // Data types are currently unique
    variant: EnumPattern,
}

impl Display for EEnumVariantId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.ident, self.variant)
    }
}
