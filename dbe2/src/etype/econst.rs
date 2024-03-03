use crate::value::{ENumber, EValue};
use serde::{Deserialize, Serialize};
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
