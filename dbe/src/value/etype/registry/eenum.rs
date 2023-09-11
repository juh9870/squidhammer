use crate::value::etype::registry::serialization::parse_type;
use crate::value::etype::registry::{EObjectType, ETypesRegistry, ETypetId};
use crate::value::etype::{EDataType, ETypeConst};
use crate::value::{EValue, JsonValue};
use anyhow::{anyhow, bail, Context};
use itertools::Itertools;
use serde_json::Value;
use std::fmt::{Display, Formatter};
use ustr::Ustr;

#[derive(Debug, Clone, Eq, PartialEq)]
enum EnumPattern {
    Field(Ustr, ETypeConst),
}

impl EnumPattern {
    fn deserialize(data: &JsonValue) -> anyhow::Result<Self> {
        match data {
            Value::Object(fields) => {
                let (k, v) = fields
                    .iter()
                    .exactly_one()
                    .map_err(|_| anyhow!("Exactly one field was expected in pattern definition"))?;
                let k = Ustr::from(k);
                let v = parse_type(v)?;

                let EDataType::Const { value } = v else {
                    bail!("Patterns only support constant values")
                };
                Ok(Self::Field(k, value))
            }
            _ => bail!("Non-object patterns are not supported"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EEnumVariant {
    pat: EnumPattern,
    data: ETypetId,
}

impl EEnumVariant {
    pub fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        registry.default_value(&self.data)
    }

    pub fn deserialize(
        registry: &mut ETypesRegistry,
        item: &JsonValue,
    ) -> anyhow::Result<EEnumVariant> {
        let (target_type, pat) = {
            match item {
                Value::String(ty) => (ty.as_str(), None),
                Value::Object(obj) => {
                    let target_type = obj
                        .get("type")
                        .context("Mandatory field `type` is missing")?
                        .as_str()
                        .context("`type` field must be a string")?;

                    (target_type, obj.get("pattern"))
                }
                _ => {
                    bail!("Expected enum definition item to be an object or a string")
                }
            }
        };

        let target_id = match parse_type(&JsonValue::String(target_type.to_string()))
            .context("While parsing enum item type")?
        {
            EDataType::Object { ident } => ident,
            _ => bail!("Enum type must be a type reference"),
        };

        registry.assert_defined(&target_id)?;

        let pat = if let Some(pattern) = pat {
            EnumPattern::deserialize(pattern).context("While parsing pattern")?
        } else {
            let target_type = registry
                .fetch_or_deserialize(target_id)
                .with_context(|| format!("Error during automatic pattern detection\n> *If you see recursion error at the top of this log, consider specifying `pattern` field manually*"))?;

            match target_type {
                EObjectType::Enum(_) => bail!("Automatic pattern detection only works with struct targets, but `{target_id}` is an enum. Please specify `pattern` manually"),
                EObjectType::Struct(data) => {
                    let pat = data.fields.iter().filter_map(|f| {
                        match f.ty {
                            EDataType::Const { value } => {
                                Some((f.name, value))
                            }
                            _=> None,
                        }
                    }).exactly_one().map_err(|_|anyhow!("Target struct `{target_id}` contains multiple constant fields. Please specify `pattern` manually"))?;

                    EnumPattern::Field(pat.0,pat.1)
                }
            }
        };
        Ok(EEnumVariant {
            pat,
            data: target_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct EEnumData {
    pub ident: ETypetId,
    pub variants: Vec<EEnumVariant>,
}

impl EEnumData {
    pub fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        let default_variant = self.variants.first().expect("Expect enum to not be empty");
        EValue::Enum {
            ident: EEnumVariantId {
                ident: self.ident,
                variant: default_variant.data,
            },
            data: Box::new(default_variant.default_value(registry)),
        }
    }

    pub fn deserialize(
        registry: &mut ETypesRegistry,
        id: ETypetId,
        data: &Vec<JsonValue>,
    ) -> anyhow::Result<EEnumData> {
        anyhow::ensure!(!data.is_empty(), "Enum must have at least one variant");
        let mut items = Vec::with_capacity(data.len());
        for (i, item) in data.iter().enumerate() {
            let item = EEnumVariant::deserialize(registry, item)
                .with_context(|| format!("Parsing enum item at position {i}"))?;
            items.push(item);
        }
        Ok(EEnumData {
            variants: items,
            ident: id,
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EEnumVariantId {
    ident: ETypetId,
    variant: ETypetId,
}

impl Display for EEnumVariantId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.ident, self.variant)
    }
}
