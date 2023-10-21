use crate::value::etype::registry::eitem::{EItemType, EItemTypeTrait};
use crate::value::etype::registry::{ETypeId, ETypesRegistry};
use crate::value::EValue;
use anyhow::Context;
use ustr::{Ustr, UstrMap};

#[derive(Debug, Clone)]
pub struct EStructField {
    pub name: Ustr,
    pub ty: EItemType,
}

#[derive(Debug, Clone)]
pub struct EStructData {
    pub generic_arguments: Vec<Ustr>,
    pub ident: ETypeId,
    pub fields: Vec<EStructField>,
}

impl EStructData {
    pub fn new(ident: ETypeId) -> EStructData {
        Self {
            generic_arguments: vec![],
            fields: Default::default(),
            ident,
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

    pub fn apply_generics(&self, arguments: &UstrMap<EItemType>) -> anyhow::Result<Self> {
        let mut cloned = self.clone();
        for x in &mut cloned.fields {
            if let EItemType::Generic(g) = &x.ty {
                let item = arguments.get(&g.argument_name).with_context(|| {
                    format!("Generic argument `{}` is not provided", g.argument_name)
                })?;
                x.ty = item.clone();
            }
        }

        Ok(cloned)
    }
}
