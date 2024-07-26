use crate::etype::econst::ETypeConst;
use crate::json_utils::JsonValue;
use crate::value::id::ETypeId;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use ustr::Ustr;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum EnumPattern {
    StructField(Ustr, ETypeConst),
    Boolean,
    Number,
    String,
    Ref(ETypeId),
    Const(ETypeConst),
    List,
    Map,
    Never,
}

impl Display for EnumPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EnumPattern::StructField(field, ty) => write!(f, "{{\"{field}\": \"{ty}\"}}"),
            EnumPattern::Boolean => write!(f, "{{boolean}}"),
            EnumPattern::Number => write!(f, "{{number}}"),
            EnumPattern::String => write!(f, "{{string}}"),
            EnumPattern::Const(ty) => write!(f, "{{{ty}}}"),
            EnumPattern::Ref(ty) => write!(f, "{{Ref<{ty}>}}"),
            EnumPattern::List => write!(f, "{{list}}"),
            EnumPattern::Map => write!(f, "{{map}}"),
            EnumPattern::Never => write!(f, "{{never}}"),
        }
    }
}

impl EnumPattern {
    pub fn matches_json(&self, value: &JsonValue) -> bool {
        match self {
            EnumPattern::StructField(field, c) => value.as_object().is_some_and(|m| {
                m.get(field.as_str())
                    .is_some_and(|val| c.matches_json(val).by_value)
            }),
            EnumPattern::Boolean => value.is_boolean(),
            EnumPattern::Number => value.is_number(),
            EnumPattern::String => value.is_string(),
            EnumPattern::Ref(_) => value.is_string(),
            EnumPattern::Const(c) => c.matches_json(value).by_value,
            EnumPattern::List => value.is_array(),
            EnumPattern::Map => value.is_object(),
            EnumPattern::Never => false,
        }
    }
}
