use crate::etype::econst::ETypeConst;
use crate::etype::eenum::variant::EEnumVariantId;
use crate::etype::EDataType;
use crate::value::id::{EListId, EMapId, ETypeId};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

use crate::json_utils::JsonValue;
use crate::m_try;
use crate::registry::ETypesRegistry;
use miette::{bail, miette, Context};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use strum::{EnumDiscriminants, EnumIs};
use ustr::Ustr;

pub mod id;

pub type ENumber = OrderedFloat<f64>;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumDiscriminants,
    Ord,
    PartialOrd,
    EnumIs,
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
    (primitive $type:tt, $result:ty, $name:tt, $field_name:ident) => {
        impl From<$result> for EValue {
            fn from(value: $result) -> Self {
                Self::$type { value }
            }
        }

        try_to!($type, $result, $name, $field_name);
    };
    ($type:tt, $result:ty, $name:tt, $field_name:ident) => {
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

            impl EValue {
                pub fn [<try_into_ $name>](self) -> miette::Result<$result> {
                    if let EValue::$type { $field_name, .. } = self {
                        Ok($field_name)
                    } else {
                        miette::bail!(
                            "invalid cast from {:?} to {}",
                            self,
                            stringify!($name)
                            // rust_i18n::t!(stringify!($name))
                        )
                    }
                }
                pub fn [<try_as_ $name>](&self) -> miette::Result<&$result> {
                    if let EValue::$type { $field_name, .. } = self {
                        Ok(&$field_name)
                    } else {
                        miette::bail!(
                            "invalid cast from {:?} to {}",
                            self,
                            stringify!($name)
                            // rust_i18n::t!(stringify!($name))
                        )
                    }
                }

                pub fn [<try_as_ $name _mut>](&mut self) -> miette::Result<&mut $result> {
                    if let EValue::$type { $field_name, .. } = self {
                        Ok($field_name)
                    } else {
                        miette::bail!(
                            "invalid cast from {:?} to {}",
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

try_to!(primitive Number, ENumber, number, value);
try_to!(primitive Boolean, bool, boolean, value);
try_to!(primitive String, String, string, value);
try_to!(Struct, BTreeMap<Ustr, EValue>, struct, fields);

impl EValue {
    pub fn try_get_field(&self, field: &str) -> miette::Result<&EValue> {
        if let EValue::Struct { fields, .. } = self {
            fields
                .get(&Ustr::from(field))
                .ok_or_else(|| miette!("field `{}` not found", field))
        } else {
            bail!("expected struct, got {:?}", self)
        }
    }
}

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
                    values.iter().map(ToString::to_string).join(", ")
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

impl EValue {
    pub fn write_json(&self, registry: &ETypesRegistry) -> miette::Result<JsonValue> {
        let value = match self {
            EValue::Null => JsonValue::Null,
            EValue::Boolean { value } => JsonValue::Bool(*value),
            EValue::Number { value } => JsonValue::from(value.0),
            EValue::String { value } => JsonValue::from(value.clone()),
            EValue::Struct { ident, fields } => m_try(|| {
                let struct_data = registry
                    .get_struct(ident)
                    .ok_or_else(|| miette!("unknown struct `{}`", ident))?;

                struct_data.write_json(fields, registry)
            })
            .with_context(|| format!("in struct `{}`", ident))?,
            EValue::Enum { data, variant } => m_try(|| {
                let enum_data = registry
                    .get_enum(&variant.enum_id())
                    .ok_or_else(|| miette!("unknown enum `{}`", variant.enum_id()))?;

                enum_data.write_json(registry, data, variant)
            })
            .with_context(|| format!("in enum variant `{}`", variant.variant_name()))?,
            EValue::List { id: _, values } => JsonValue::Array(
                values
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        e.write_json(registry)
                            .with_context(|| format!("at index {}", i))
                    })
                    .try_collect()?,
            ),
            EValue::Map { id: _, values } => JsonValue::Object(
                values
                    .iter()
                    .map(|(k, v)| {
                        let key_val = k.write_json(registry)?;
                        let JsonValue::String(key) = key_val else {
                            bail!(
                                "json map key should serialize to string, but instead got {}",
                                key_val
                            );
                        };
                        Ok((
                            key,
                            v.write_json(registry)
                                .with_context(|| format!("in entry with key `{k}`"))?,
                        ))
                    })
                    .try_collect()?,
            ),
        };

        Ok(value)
    }
}

macro_rules! estruct {
    (
        $ident:tt {
            $($field_name:tt : $field_ty:expr),* $(,)?
        }
    ) => {
        {
            let mut fields = std::collections::BTreeMap::<ustr::Ustr, $crate::value::EValue>::default();
            $(
                fields.insert(ustr::Ustr::from($crate::value::estruct_keyish!($field_name)), $crate::value::EValue::from($field_ty));
            )*
            $crate::value::EValue::Struct {
                ident: $crate::value::estruct_keyish!($ident),
                fields,
            }
        }
    };
}

macro_rules! estruct_keyish {
    ($a:ident) => {
        $a
    };
    ($a:expr) => {
        $a
    };
    ($a:literal) => {
        $a
    };
}

pub(crate) use estruct;
pub(crate) use estruct_keyish;
