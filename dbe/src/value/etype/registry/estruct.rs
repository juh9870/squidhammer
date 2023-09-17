use anyhow::Context;
use serde_json::Map;
use ustr::Ustr;

use utils::somehow;

use crate::value::etype::registry::serialization::parse_type;
use crate::value::etype::registry::{ETypesRegistry, ETypetId};
use crate::value::etype::EDataType;
use crate::value::{EValue, JsonValue};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EStructField {
    pub name: Ustr,
    pub ty: EDataType,
}

#[derive(Debug, Clone)]
pub struct EStructData {
    pub ident: ETypetId,
    pub fields: Vec<EStructField>,
}

impl EStructData {
    pub fn new(ident: ETypetId) -> EStructData {
        Self {
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
                .map(|f| (f.name, f.ty.default_value(registry)))
                .collect(),
        }
    }
    pub fn deserialize(
        registry: &mut ETypesRegistry,
        id: ETypetId,
        fields: &Map<String, JsonValue>,
    ) -> anyhow::Result<EStructData> {
        let mut data = EStructData::new(id);
        for (name, value) in fields {
            somehow!({
                let name_ident = Ustr::from(name);
                let ty = parse_type(value)?;

                if let EDataType::Object { ident } = &ty {
                    registry.assert_defined(ident)?;
                }

                data.fields.push(EStructField {
                    name: name_ident,
                    ty,
                });
            })
            .with_context(|| format!("While parsing field \"{name}\""))?;
        }

        Ok(data)
    }
}
