use crate::etype::eitem::EItemType;
use crate::etype::EDataType;
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use crate::value::EValue;
use std::collections::BTreeMap;

use crate::json_utils::repr::Repr;
use crate::json_utils::{json_kind, JsonValue};
use miette::{bail, miette, Context};
use ustr::{Ustr, UstrMap};

#[derive(Debug, Clone)]
pub struct EStructData {
    pub generic_arguments: Vec<Ustr>,
    pub ident: ETypeId,
    pub fields: Vec<EStructField>,
    pub id_field: Option<usize>,
    pub repr: Option<Repr>,
}

#[derive(Debug, Clone)]
pub struct EStructField {
    pub name: Ustr,
    pub ty: EItemType,
}

impl EStructData {
    pub fn new(ident: ETypeId, generic_arguments: Vec<Ustr>, repr: Option<Repr>) -> EStructData {
        Self {
            generic_arguments,
            fields: Default::default(),
            ident,
            id_field: None,
            repr,
        }
    }

    pub fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        EValue::Struct {
            ident: self.ident,
            fields: self
                .fields
                .iter()
                .map(|f| (f.name.as_str().into(), f.ty.default_value(registry)))
                .collect(),
        }
    }
    pub fn apply_generics(
        mut self,
        arguments: &UstrMap<EItemType>,
        new_id: ETypeId,
        _registry: &mut ETypesRegistry,
    ) -> miette::Result<Self> {
        self.ident = new_id;
        for x in &mut self.fields {
            if let EItemType::Generic(g) = &x.ty {
                let item = arguments.get(&g.argument_name).ok_or_else(|| {
                    miette!("generic argument `{}` is not provided", g.argument_name)
                })?;
                x.ty = item.clone();
            }
        }

        // if let Ok((_, item)) = arguments.iter().exactly_one() {
        //     if self.color.is_none() {
        //         self.color = Some(item.ty().color(registry));
        //     }
        // }

        self.generic_arguments = vec![];

        Ok(self)
    }

    pub fn id_field(&self) -> Option<&EStructField> {
        self.id_field.map(|i| &self.fields[i])
    }

    pub fn id_field_data(&self) -> Option<ETypeId> {
        self.id_field().map(|e| {
            if let EDataType::Id { ty } = &e.ty.ty() {
                *ty
            } else {
                panic!("Bad struct state")
            }
        })
    }

    pub(crate) fn add_field(&mut self, field: EStructField) -> miette::Result<()> {
        if let EDataType::Id { ty } = &field.ty.ty() {
            if self.id_field.is_some() {
                bail!("struct already has an ID field");
            }
            if ty != &self.ident {
                bail!("struct can't have an ID field with different type. Expected {}, but got {} instead", self.ident, ty)
            }
            self.id_field = Some(self.fields.len());
        }
        self.fields.push(field);

        Ok(())
    }

    pub fn parse_json(
        &self,
        registry: &ETypesRegistry,
        mut data: JsonValue,
    ) -> miette::Result<EValue> {
        let Some(data) = data.as_object_mut() else {
            bail!(
                "invalid data type. Expected object but got `{}`",
                json_kind(&data)
            )
        };

        let mut fields = BTreeMap::<Ustr, EValue>::default();

        for field in &self.fields {
            let value: EValue = if let Some(json_value) = data.remove(field.name.as_str()) {
                field
                    .ty
                    .ty()
                    .parse_json(registry, json_value)
                    .with_context(|| format!("in field `{}`", field.name))?
            } else {
                field.ty.default_value(registry)
            };

            fields.insert(field.name, value);
        }

        Ok(EValue::Struct {
            ident: self.ident,
            fields,
        })
    }
}
