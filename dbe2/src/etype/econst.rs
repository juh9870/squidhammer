use crate::json_utils::JsonValue;
use crate::value::{ENumber, EValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::{Display, Formatter};
use ustr::Ustr;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ETypeConst {
    String(Ustr),
    Number(ENumber),
    Boolean(bool),
    Null,
}

impl ETypeConst {
    pub fn default_value(&self) -> EValue {
        match self {
            ETypeConst::Boolean(value) => (*value).into(),
            ETypeConst::Number(value) => (*value).into(),
            ETypeConst::String(value) => value.to_string().into(),
            ETypeConst::Null => EValue::Null,
        }
    }

    /// Checks whenever the provided JSON matches the constant
    pub fn matches_json(&self, data: &JsonValue) -> ConstJsonMatchResult {
        #[inline(always)]
        fn m(ty: bool, value: bool) -> ConstJsonMatchResult {
            ConstJsonMatchResult {
                by_type: ty,
                by_value: value,
            }
        }

        match (data, self) {
            (Value::Null, Self::Null) => m(true, true),
            (Value::Bool(v1), Self::Boolean(v2)) => m(true, v1 == v2),
            (Value::Number(n1), Self::Number(n2)) => m(true, n1.as_f64().unwrap() == n2.0),
            (Value::String(str1), Self::String(str2)) => m(true, str1.as_str() == str2.as_str()),
            _ => m(false, false),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ConstJsonMatchResult {
    pub by_type: bool,
    pub by_value: bool,
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

impl From<ENumber> for ETypeConst {
    fn from(value: ENumber) -> Self {
        ETypeConst::Number(value)
    }
}

impl From<f64> for ETypeConst {
    fn from(value: f64) -> Self {
        ETypeConst::Number(value.into())
    }
}

impl From<bool> for ETypeConst {
    fn from(value: bool) -> Self {
        ETypeConst::Boolean(value)
    }
}

impl From<Ustr> for ETypeConst {
    fn from(value: Ustr) -> Self {
        ETypeConst::String(value)
    }
}
