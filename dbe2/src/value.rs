use crate::etype::econst::ETypeConst;
use crate::etype::eenum::variant::EEnumVariantId;
use crate::etype::EDataType;
use crate::value::id::{EListId, EMapId, ETypeId};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use strum::EnumDiscriminants;
use ustr::Ustr;

pub mod id;

pub type ENumber = OrderedFloat<f64>;

#[derive(
    Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, EnumDiscriminants, Ord, PartialOrd,
)]
#[strum_discriminants(derive(Ord, PartialOrd), vis())]
#[serde(tag = "type")]
pub enum EValue {
    Null,
    Boolean {
        value: bool,
    },
    Number {
        value: ENumber,
    },
    String {
        value: String,
    },
    Struct {
        ident: ETypeId,
        fields: BTreeMap<Ustr, EValue>,
    },
    Enum {
        variant: EEnumVariantId,
        data: Box<EValue>,
    },
    List {
        id: EListId,
        values: Vec<EValue>,
    },
    Map {
        id: EMapId,
        values: BTreeMap<EValue, EValue>,
    },
}

impl EValue {
    pub fn ty(&self) -> EDataType {
        match self {
            EValue::Null => EDataType::Const {
                value: ETypeConst::Null,
            },
            EValue::Boolean { .. } => EDataType::Boolean,
            EValue::Number { .. } => EDataType::Number,
            EValue::String { .. } => EDataType::String,
            EValue::Struct { ident, .. } => EDataType::Object { ident: *ident },
            EValue::Enum { variant, .. } => EDataType::Object {
                ident: variant.enum_id(),
            },
            EValue::List { id: ty, .. } => EDataType::List { id: *ty },
            EValue::Map { id, .. } => EDataType::Map { id: *id },
        }
    }
}

macro_rules! try_to {
    ($type:tt, $result:ty, $name:ident) => {
        paste::item! {
            impl TryFrom<EValue> for $result {
                type Error = miette::Error;

                fn try_from(value: EValue) -> Result<Self, Self::Error> {
                    value.[<try_into_ $name>]()
                }
            }

            impl TryFrom<&EValue> for $result {
                type Error = miette::Error;

                fn try_from(value: &EValue) -> Result<Self, Self::Error> {
                    value.[<try_as_ $name>]().map(|e|e.clone())
                }
            }

            impl <'a> TryFrom<&'a EValue> for &'a $result {
                type Error = miette::Error;

                fn try_from(value: &'a EValue) -> Result<Self, Self::Error> {
                    value.[<try_as_ $name>]()
                }
            }

            impl <'a> TryFrom<&'a mut EValue> for &'a mut $result {
                type Error = miette::Error;

                fn try_from(value: &'a mut EValue) -> Result<Self, Self::Error> {
                    value.[<try_as_ $name _mut>]()
                }
            }

            // impl<'a> TryFrom<EValueInputWrapper<'a>> for $result {
            //     type Error = miette::Error;
            //
            //     fn try_from(value: EValueInputWrapper<'a>) -> Result<Self, Self::Error> {
            //         if value.0.len() != 1 {
            //             miette::bail!("Got {} inputs where only one was expected.", value.0.len());
            //         }
            //
            //         Self::try_from(value.0[0])
            //     }
            // }

            impl From<$result> for EValue {
                fn from(value: $result) -> Self {
                    Self::$type{value}
                }
            }

            impl EValue {
                pub fn [<try_into_ $name>](self) -> miette::Result<$result> {
                    if let EValue::$type { value } = self {
                        Ok(value)
                    } else {
                        miette::bail!(
                            "Invalid cast from {:?} to {}",
                            self,
                            stringify!($name)
                            // rust_i18n::t!(stringify!($name))
                        )
                    }
                }
                pub fn [<try_as_ $name>](&self) -> miette::Result<&$result> {
                    if let EValue::$type { value } = self {
                        Ok(&value)
                    } else {
                        miette::bail!(
                            "Invalid cast from {:?} to {}",
                            self,
                            stringify!($name)
                            // rust_i18n::t!(stringify!($name))
                        )
                    }
                }

                pub fn [<try_as_ $name _mut>](&mut self) -> miette::Result<&mut $result> {
                    if let EValue::$type { value } = self {
                        Ok(value)
                    } else {
                        miette::bail!(
                            "Invalid cast from {:?} to {}",
                            self,
                            stringify!($name)
                            // rust_i18n::t!(stringify!($name))
                        )
                    }
                }
            }
        }
    };
}

try_to!(Number, ENumber, number);
try_to!(Boolean, bool, boolean);
try_to!(String, String, string);

impl From<f64> for EValue {
    fn from(value: f64) -> Self {
        Self::Number {
            value: OrderedFloat(value),
        }
    }
}
impl From<&f64> for EValue {
    fn from(value: &f64) -> Self {
        Self::Number {
            value: OrderedFloat(*value),
        }
    }
}

impl TryFrom<EValue> for f64 {
    type Error = miette::Error;

    fn try_from(value: EValue) -> Result<Self, Self::Error> {
        value.try_as_number().copied().map(|n| n.0)
    }
}

impl TryFrom<&EValue> for f64 {
    type Error = miette::Error;

    fn try_from(value: &EValue) -> Result<Self, Self::Error> {
        value.try_as_number().copied().map(|n| n.0)
    }
}
impl From<f32> for EValue {
    fn from(value: f32) -> Self {
        Self::Number {
            value: OrderedFloat(value as f64),
        }
    }
}
impl From<&f32> for EValue {
    fn from(value: &f32) -> Self {
        Self::Number {
            value: OrderedFloat(*value as f64),
        }
    }
}

impl TryFrom<EValue> for f32 {
    type Error = miette::Error;

    fn try_from(value: EValue) -> Result<Self, Self::Error> {
        value.try_as_number().copied().map(|n| n.0 as f32)
    }
}

impl TryFrom<&EValue> for f32 {
    type Error = miette::Error;

    fn try_from(value: &EValue) -> Result<Self, Self::Error> {
        value.try_as_number().copied().map(|n| n.0 as f32)
    }
}

impl Display for EValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EValue::Boolean { value } => write!(f, "{value}"),
            EValue::Number { value } => write!(f, "{value}"),
            EValue::String { value } => write!(f, "\"{value}\""),
            EValue::Null => write!(f, "null"),
            EValue::Struct { ident, fields } => {
                write!(
                    f,
                    "{ident}{{{}}}",
                    fields
                        .iter()
                        .map(|(field, value)| format!("\"{field}\": {value}"))
                        .join(", ")
                )
            }
            EValue::Enum {
                variant: ident,
                data,
            } => {
                write!(f, "{ident}({data})")
            }
            EValue::List { id, values } => {
                write!(
                    f,
                    "{id}[{}]",
                    values.iter().map(|e| e.to_string()).join(", ")
                )
            }
            EValue::Map { id, values } => {
                write!(
                    f,
                    "{id}{{{}}}",
                    values.iter().map(|(k, v)| format!("{k}: {v}")).join(", ")
                )
            }
        }
    }
}
