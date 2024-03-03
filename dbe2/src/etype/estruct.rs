use crate::etype::eitem::EItemType;
use crate::etype::EDataType;
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use crate::value::EValue;

use miette::{bail, miette};
use ustr::{Ustr, UstrMap};

#[derive(Debug, Clone)]
pub struct EStructData {
    pub generic_arguments: Vec<Ustr>,
    pub ident: ETypeId,
    pub fields: Vec<EStructField>,
    pub id_field: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct EStructField {
    pub name: Ustr,
    pub ty: EItemType,
}

impl EStructData {
    pub fn new(ident: ETypeId, generic_arguments: Vec<Ustr>) -> EStructData {
        Self {
            generic_arguments,
            fields: Default::default(),
            ident,
            id_field: None,
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
                    miette!("Generic argument `{}` is not provided", g.argument_name)
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
                bail!("Struct already has an ID field");
            }
            if ty != &self.ident {
                bail!("Struct can't have an ID field with different type")
            }
            self.id_field = Some(self.fields.len());
        }
        self.fields.push(field);

        Ok(())
    }
}
