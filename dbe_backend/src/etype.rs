use crate::etype::default::DefaultEValue;
use crate::etype::econst::ETypeConst;
use crate::etype::eitem::EItemInfo;
use crate::etype::eobject::EObject;
use crate::json_utils::{json_expected, json_kind, JsonValue};
use crate::m_try;
use crate::registry::{EObjectType, ETypesRegistry};
use crate::value::id::{EListId, EMapId, ETypeId};
use crate::value::EValue;
use miette::{bail, miette, Context};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::LazyLock;
use strum::EnumIs;
use ustr::Ustr;
use utils::whatever_ref::{WhateverRef, WhateverRefMap};

pub mod conversion;
pub mod default;
pub mod econst;
pub mod eenum;
pub mod eitem;
pub mod eobject;
pub mod estruct;
pub mod property;
pub mod title;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIs)]
pub enum EDataType {
    /// Primitive boolean type
    Boolean,
    /// Primitive numeric type
    Number,
    /// Primitive string type
    String,
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
    /// Unknown type. Cannot be named by the kdl type system, but can be used
    /// in graphs
    Unknown,
}

impl EDataType {
    pub fn default_value(&self, reg: &ETypesRegistry) -> DefaultEValue {
        match self {
            EDataType::Boolean => EValue::Boolean { value: false },
            EDataType::Number => EValue::Number { value: 0.0.into() },
            EDataType::String => EValue::String {
                value: Default::default(),
            },
            EDataType::Object { ident } => return reg.default_value_inner(ident),
            EDataType::Const { value } => value.default_value(),
            EDataType::List { id } => EValue::List {
                id: *id,
                values: vec![],
            },
            EDataType::Map { id } => EValue::Map {
                id: *id,
                values: Default::default(),
            },
            EDataType::Unknown => EValue::Null,
        }
        .into()
    }

    pub const fn null() -> EDataType {
        EDataType::Const {
            value: ETypeConst::Null,
        }
    }

    /// Returns the name of the type for debugging purposes
    pub fn name(&self) -> Cow<'_, str> {
        match self {
            EDataType::Boolean => "boolean".into(),
            EDataType::Number => "number".into(),
            EDataType::String => "string".into(),
            EDataType::Object { ident } => ident.to_string().into(),
            EDataType::Const { value } => value.to_string().into(),
            EDataType::List { id: ty } => ty.to_string().into(),
            EDataType::Map { id: ty } => ty.to_string().into(),
            EDataType::Unknown => "unknown".into(),
        }
    }

    /// Returns the human-readable title of the type
    pub fn title(&self, registry: &ETypesRegistry) -> String {
        match self {
            EDataType::Boolean | EDataType::Number | EDataType::String | EDataType::Unknown => {
                self.name().to_string()
            }
            EDataType::Object { ident } => registry.get_object(ident).map_or_else(
                || format!("Unknown object `{}`", ident),
                |data| data.title(registry),
            ),
            EDataType::Const { value } => value.to_string(),
            EDataType::List { id } => registry.get_list(id).map_or_else(
                || format!("Unknown list `{}`", id),
                |data| format!("List<{}>", data.value_type.title(registry)),
            ),
            EDataType::Map { id } => registry.get_map(id).map_or_else(
                || format!("Unknown map `{}`", id),
                |data| {
                    format!(
                        "Map<{}, {}>",
                        data.key_type.title(registry),
                        data.value_type.title(registry)
                    )
                },
            ),
        }
    }

    /// Returns the generic arguments names for this type
    pub fn generic_arguments_names<'a>(
        &self,
        registry: &'a ETypesRegistry,
    ) -> impl Deref<Target = [Ustr]> + 'a {
        enum Names<'a> {
            Ref(&'a [Ustr]),
            Map(WhateverRefMap<'a, EObjectType, [Ustr]>),
        }

        impl Deref for Names<'_> {
            type Target = [Ustr];
            fn deref(&self) -> &Self::Target {
                match self {
                    Names::Ref(r) => r,
                    Names::Map(r) => r.deref(),
                }
            }
        }

        match self {
            EDataType::Boolean
            | EDataType::Number
            | EDataType::String
            | EDataType::Const { .. }
            | EDataType::Unknown => Names::Ref(&[]),
            EDataType::Object { ident } => {
                let obj = registry.get_object(ident).expect("object should exist");
                Names::Map(WhateverRef::map(obj, |obj| obj.generic_arguments_names()))
            }
            EDataType::List { .. } => {
                static NAMES: LazyLock<[Ustr; 1]> = LazyLock::new(|| [Ustr::from("Item")]);
                Names::Ref(NAMES.deref())
            }
            EDataType::Map { .. } => {
                static NAMES: LazyLock<[Ustr; 2]> =
                    LazyLock::new(|| [Ustr::from("Key"), Ustr::from("Item")]);
                Names::Ref(NAMES.deref())
            }
        }
    }

    /// Returns the generic arguments values for this type
    pub fn generic_arguments_values<'a>(
        &self,
        registry: &'a ETypesRegistry,
    ) -> impl Deref<Target = [EItemInfo]> + 'a {
        enum Names<'a> {
            Nil,
            Vec(SmallVec<[EItemInfo; 2]>),
            Ref(WhateverRefMap<'a, EObjectType, [EItemInfo]>),
        }

        impl Deref for Names<'_> {
            type Target = [EItemInfo];
            fn deref(&self) -> &Self::Target {
                match self {
                    Names::Nil => &[],
                    Names::Vec(v) => v.as_slice(),
                    Names::Ref(r) => r.deref(),
                }
            }
        }

        match self {
            EDataType::Boolean
            | EDataType::Number
            | EDataType::String
            | EDataType::Const { .. }
            | EDataType::Unknown => Names::Nil,
            EDataType::Object { ident } => {
                let obj = registry.get_object(ident).expect("object should exist");
                Names::Ref(WhateverRef::map(obj, |obj| obj.generic_arguments_values()))
            }
            EDataType::List { id } => {
                let list = registry.get_list(id).expect("list should exist");
                Names::Vec(smallvec![EItemInfo::simple_type(list.value_type)])
            }
            EDataType::Map { id } => {
                let map = registry.get_map(id).expect("map should exist");
                Names::Vec(smallvec![
                    EItemInfo::simple_type(map.key_type),
                    EItemInfo::simple_type(map.value_type),
                ])
            }
        }
    }

    pub fn parse_json(
        &self,
        registry: &ETypesRegistry,
        data: &mut JsonValue,
        inline: bool,
    ) -> miette::Result<EValue> {
        match self {
            EDataType::Boolean => json_expected(data.as_bool(), data, "bool").map(EValue::from),
            EDataType::Number => json_expected(data.as_number(), data, "number")
                .map(|num| OrderedFloat(num.as_f64().unwrap()).into()),
            EDataType::String => {
                json_expected(data.as_str(), data, "string").map(|s| s.to_string().into())
            }
            EDataType::Object { ident } => {
                let obj = registry
                    .get_object(ident)
                    .ok_or_else(|| miette!("object id was not present in registry: `{}`", ident))?;

                obj.parse_json(registry, data, inline)
            }
            EDataType::Const { value } => {
                let m = value.matches_json(data);

                if !m.by_type {
                    bail!(
                        "invalid data type. Expected {} but got {}",
                        value,
                        json_kind(data)
                    )
                }

                if !m.by_value {
                    bail!("invalid constant. Expected {} but got {}", value, data)
                }

                Ok(value.default_value())
            }
            EDataType::List { id } => {
                let list = registry.get_list(id).ok_or_else(|| {
                    miette!(
                        "!!INTERNAL ERROR!! list id was not present in registry: `{}`",
                        id
                    )
                })?;

                let JsonValue::Array(items) = data else {
                    bail!(
                        "invalid data type. Expected list but got {}",
                        json_kind(data)
                    )
                };

                let mut list_items = vec![];
                for (i, x) in items.iter_mut().enumerate() {
                    list_items.push(
                        list.value_type
                            .parse_json(registry, x, false)
                            .with_context(|| format!("at index {}", i))?,
                    );
                }

                Ok(EValue::List {
                    id: *id,
                    values: list_items,
                })
            }
            EDataType::Map { id } => {
                let map = registry.get_map(id).ok_or_else(|| {
                    miette!(
                        "!!INTERNAL ERROR!! map id was not present in registry: `{}`",
                        id
                    )
                })?;

                let JsonValue::Object(obj) = data else {
                    bail!(
                        "invalid data type. Expected map but got {}",
                        json_kind(data)
                    )
                };

                let mut entries = BTreeMap::new();

                for (k, v) in obj {
                    let key_name = k.clone();
                    let (k, v) = m_try(|| {
                        let k = map.key_type.parse_json(
                            registry,
                            &mut JsonValue::String(k.clone()),
                            false,
                        )?;
                        let v = map.value_type.parse_json(registry, v, false)?;
                        Ok((k, v))
                    })
                    .with_context(|| format!("in entry with key `{}`", key_name))?;

                    entries.insert(k, v);
                }

                Ok(EValue::Map {
                    id: *id,
                    values: entries,
                })
            }
            EDataType::Unknown => {
                bail!("cannot parse unknown type")
            }
        }
    }
}
