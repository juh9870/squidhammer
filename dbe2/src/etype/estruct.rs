use crate::etype::econst::ETypeConst;
use crate::etype::eitem::EItemInfo;
use crate::json_utils::repr::{JsonRepr, Repr};
use crate::json_utils::{json_kind, JsonMap, JsonValue};
use crate::m_try;
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use crate::value::EValue;
use ahash::AHashMap;
use itertools::Itertools;
use miette::{bail, miette, Context};
use std::collections::BTreeMap;
use ustr::{Ustr, UstrMap};

#[derive(Debug, Clone)]
pub struct EStructData {
    pub generic_arguments: Vec<Ustr>,
    pub generic_arguments_values: Vec<EItemInfo>,
    pub ident: ETypeId,
    pub fields: Vec<EStructField>,
    // pub id_field: Option<usize>,
    pub repr: Option<Repr>,
    pub extra_properties: AHashMap<String, ETypeConst>,
}

#[derive(Debug, Clone)]
pub struct EStructField {
    pub name: Ustr,
    pub ty: EItemInfo,
}

impl EStructField {
    fn is_inline(&self) -> bool {
        self.ty
            .extra_properties()
            .get("inline")
            .is_some_and(|val| val == &ETypeConst::Boolean(true))
    }
}

impl EStructData {
    pub fn new(
        ident: ETypeId,
        generic_arguments: Vec<Ustr>,
        repr: Option<Repr>,
        extra_properties: AHashMap<String, ETypeConst>,
    ) -> EStructData {
        Self {
            generic_arguments,
            generic_arguments_values: vec![],
            fields: Default::default(),
            ident,
            // id_field: None,
            repr,
            extra_properties,
        }
    }

    pub(crate) fn default_value_inner(&self, registry: &ETypesRegistry) -> EValue {
        EValue::Struct {
            ident: self.ident,
            fields: self
                .fields
                .iter()
                .map(|f| {
                    (
                        f.name.as_str().into(),
                        f.ty.default_value(registry).into_owned(),
                    )
                })
                .collect(),
        }
    }
    pub fn apply_generics(
        mut self,
        arguments: &UstrMap<EItemInfo>,
        new_id: ETypeId,
        _registry: &mut ETypesRegistry,
    ) -> miette::Result<Self> {
        self.ident = new_id;
        for x in &mut self.fields {
            if let EItemInfo::Generic(g) = &x.ty {
                let item = arguments.get(&g.argument_name).ok_or_else(|| {
                    miette!("generic argument `{}` is not provided", g.argument_name)
                })?;
                x.ty = item.clone();
            }
        }

        for arg in &self.generic_arguments {
            let item = arguments
                .get(arg)
                .ok_or_else(|| miette!("generic argument `{}` is not provided", arg))?;
            self.generic_arguments_values.push(item.clone());
        }

        // if let Ok((_, item)) = arguments.iter().exactly_one() {
        //     if self.color.is_none() {
        //         self.color = Some(item.ty().color(registry));
        //     }
        // }

        // self.generic_arguments = vec![];

        Ok(self)
    }

    // pub fn id_field(&self) -> Option<&EStructField> {
    //     self.id_field.map(|i| &self.fields[i])
    // }
    //
    // pub fn id_field_data(&self) -> Option<ETypeId> {
    //     self.id_field().map(|e| {
    //         if let EDataType::Id { ty } = &e.ty.ty() {
    //             *ty
    //         } else {
    //             panic!("Bad struct state")
    //         }
    //     })
    // }

    pub(crate) fn add_field(&mut self, field: EStructField) -> miette::Result<()> {
        // if let EDataType::Id { ty } = &field.ty.ty() {
        //     if self.id_field.is_some() {
        //         bail!("struct already has an ID field");
        //     }
        //     if ty != &self.ident {
        //         bail!("struct can't have an ID field with different type. Expected {}, but got {} instead", self.ident, ty)
        //     }
        //     self.id_field = Some(self.fields.len());
        // }
        self.fields.push(field);

        Ok(())
    }

    pub(crate) fn parse_json(
        &self,
        registry: &ETypesRegistry,
        json_data: &mut JsonValue,
        inline: bool,
    ) -> miette::Result<EValue> {
        if !json_data.is_object() {
            bail!(
                "invalid data type. Expected object but got `{}`",
                json_kind(json_data)
            )
        };
        #[inline(always)]
        fn j_fields(
            json_data: &mut JsonValue,
        ) -> miette::Result<&mut serde_json::map::Map<String, JsonValue>> {
            let kind = json_kind(json_data);
            let Some(data) = json_data.as_object_mut() else {
                bail!(
                    "!!INTERNAL_ERROR!! json content changed during field deserialization. Expected object but got `{}`",kind
                )
            };
            Ok(data)
        }

        let mut fields = BTreeMap::<Ustr, EValue>::default();

        for field in &self.fields {
            let data = j_fields(json_data)?;
            let value = if field.is_inline() {
                field
                    .ty
                    .ty()
                    .parse_json(registry, json_data, true)
                    .with_context(|| format!("in field `{}`", field.name))?
            } else if let Some(mut json_value) = data.remove(field.name.as_str()) {
                field
                    .ty
                    .ty()
                    .parse_json(registry, &mut json_value, false)
                    .with_context(|| format!("in field `{}`", field.name))?
            } else if let Some(default) = field.ty.extra_properties().get("default") {
                let mut json_value = default.as_json_value();
                field
                    .ty
                    .ty()
                    .parse_json(registry, &mut json_value, false)
                    .with_context(|| format!("in default value for field `{}`", field.name))?
            } else {
                field.ty.default_value(registry).into_owned()
            };
            fields.insert(field.name, value);
        }

        if !inline {
            let data = j_fields(json_data)?;
            if !data.is_empty() {
                bail!(
                    "struct contains unknown fields: {}",
                    data.keys().map(|k| format!("`{k}`")).join(", ")
                )
            }
        }

        Ok(EValue::Struct {
            ident: self.ident,
            fields,
        })
    }

    pub(crate) fn write_json(
        &self,
        fields: &BTreeMap<Ustr, EValue>,
        registry: &ETypesRegistry,
    ) -> miette::Result<JsonValue> {
        let mut json_fields = JsonMap::new();

        fn conflicting_fields(name: &str) -> miette::Report {
            miette!("multiple occurrences of field `{}` are present", name)
        }

        // TODO: throw an error if provided fields map contains unknown fields
        for field in &self.fields {
            m_try(|| {
                let json_value = if let Some(value) = fields.get(&field.name) {
                    value.write_json(registry)?
                } else {
                    field.ty.default_value(registry).write_json(registry)?
                };
                if field.is_inline() {
                    if let JsonValue::Object(obj) = json_value {
                        for (k, v) in obj {
                            if json_fields.contains_key(&k) {
                                bail!(conflicting_fields(&k))
                            } else {
                                json_fields.insert(k, v);
                            }
                        }
                    } else {
                        bail!(
                            "inline field must serialize into an object, but got `{}`",
                            json_kind(&json_value)
                        )
                    }
                } else {
                    let key = field.name.as_str().into();
                    if json_fields.contains_key(&key) {
                        bail!(conflicting_fields(field.name.as_str()))
                    }
                    json_fields.insert(key, json_value);
                }

                Ok(())
            })
            .with_context(|| format!("in field `{}`", field.name))?;
        }

        let json = JsonValue::Object(json_fields);

        if let Some(repr) = &self.repr {
            return repr.into_repr(registry, json);
        }

        Ok(json)
    }
}
