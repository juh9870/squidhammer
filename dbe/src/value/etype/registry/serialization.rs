use anyhow::{bail, Context};
use itertools::Itertools;
use knuffel::ast::{Literal, TypeName};
use knuffel::errors::DecodeError;
use knuffel::span::Spanned;
use knuffel::traits::ErrorSpan;
use knuffel::DecodeScalar;
use std::borrow::Cow;

use super::estruct::{EStructFieldDependencies, EStructFieldType};
use crate::value::etype::registry::eenum::{EEnumData, EEnumVariant, EnumPattern};
use crate::value::etype::registry::estruct::{EStructData, EStructField};
use crate::value::etype::registry::{EObjectType, ETypesRegistry, ETypetId};
use crate::value::etype::{EDataType, ETypeConst};
use crate::value::ENumber;

// pub fn parse_type(value: &Value) -> anyhow::Result<EDataType> {
//     let value = value
//         .as_str()
//         .ok_or_else(|| anyhow!("Type definition must be a string"))?;
//
//     anyhow::ensure!(!value.is_empty(), "Empty type name");
//
//     parse_type_string(value)
// }

pub fn deserialize_thing(
    registry: &mut ETypesRegistry,
    id: ETypetId,
    data: &str,
) -> Result<EObjectType, anyhow::Error> {
    let thing = knuffel::parse::<ThingTop>(&id.to_string(), data)?.value;
    Ok(match thing {
        ThingVariant::Enum(value) => EObjectType::Enum(value.into_eenum(registry, id)?),
        ThingVariant::Struct(value) => EObjectType::Struct(value.into_estruct(registry, id)?),
    })
}

#[derive(Debug, knuffel::Decode)]
struct ThingTop {
    #[knuffel(child)]
    value: ThingVariant,
}

#[derive(Debug, knuffel::Decode)]
enum ThingVariant {
    Enum(ThingEnum),
    Struct(ThingStruct),
}

#[derive(Debug, knuffel::Decode)]
struct ThingStruct {
    #[knuffel(children)]
    pub fields: Vec<EStructField>,
}

impl ThingStruct {
    fn into_estruct(
        self,
        registry: &mut ETypesRegistry,
        id: ETypetId,
    ) -> anyhow::Result<EStructData> {
        let mut data = EStructData::new(id);
        for x in self.fields {
            x.check_dependencies(registry)
                .with_context(|| format!("While deserializing field \"{}\"", x.name()))?;
            data.fields.push(x);
        }

        Ok(data)
    }
}

#[derive(Debug, knuffel::Decode)]
struct ThingEnum {
    variants: Vec<ThingEnumVariant>,
}

impl ThingEnum {
    fn into_eenum(self, registry: &mut ETypesRegistry, id: ETypetId) -> anyhow::Result<EEnumData> {
        Ok(EEnumData {
            ident: id,
            variants: self
                .variants
                .into_iter()
                .map(|e| e.into_variant(registry))
                .try_collect()?,
        })
    }
}

#[derive(Debug, knuffel::Decode)]
enum ThingEnumVariant {
    Number(#[knuffel(argument)] String),
    String(#[knuffel(argument)] String),
    Boolean(#[knuffel(argument)] String),
    Const(
        #[knuffel(argument)] String,     // variant name
        #[knuffel(argument)] ETypeConst, // value
    ),
    Struct(
        #[knuffel(argument)] String,                     // variant name
        #[knuffel(argument, str)] ETypetId,              // field name
        #[knuffel(children)] Vec<ThingEnumFieldPattern>, // pattern
    ),
}

#[derive(Debug, knuffel::Decode)]
struct ThingEnumFieldPattern {
    #[knuffel(node_name)]
    name: String,
    #[knuffel(argument)]
    value: ETypeConst,
}

impl ThingEnumVariant {
    fn into_variant(self, registry: &mut ETypesRegistry) -> anyhow::Result<EEnumVariant> {
        match self {
            ThingEnumVariant::Number(name) => Ok(EEnumVariant::scalar(name)),
            ThingEnumVariant::String(name) => Ok(EEnumVariant::string(name)),
            ThingEnumVariant::Boolean(name) => Ok(EEnumVariant::boolean(name)),
            ThingEnumVariant::Const(name, value) => Ok(EEnumVariant::econst(name, value)),
            ThingEnumVariant::Struct(name, target_id, patterns) => {
                registry.assert_defined(&target_id)?;
                let pat = if patterns.len() == 0 {
                    let target_type = registry
                        .fetch_or_deserialize(target_id)
                        .context("Error during automatic pattern detection\n> If you see recursion error at the top of this log, consider specifying pattern manually")?;

                    match target_type {
                        EObjectType::Enum(_) => {
                            bail!("Enum variant can't be an another enum")
                        }
                        EObjectType::Struct(data) => {
                            let pat = data.fields.iter().filter_map(|f| {
                                match f {
                                    EStructField::Const(field) => {

                                        Some((field.name(), field.value()))
                                    }
                                    _ => None,
                                }
                            }).exactly_one().map_err(|_| anyhow::anyhow!("Target struct `{target_id}` contains multiple constant fields. Please specify pattern manually"))?;

                            EnumPattern::StructField(pat.0, pat.1)
                        }
                    }
                } else if patterns.len() == 1 {
                    let pat = &patterns[0];
                    EnumPattern::StructField(pat.name.as_str().into(), pat.value)
                } else {
                    bail!("Multiple pattern fields are not supported")
                };

                Ok(EEnumVariant::new(
                    name,
                    pat,
                    EDataType::Object { ident: target_id },
                ))
            }
        }
    }
}

impl<S: ErrorSpan> DecodeScalar<S> for ETypeConst {
    fn type_check(
        _type_name: &Option<Spanned<TypeName, S>>,
        _ctx: &mut knuffel::decode::Context<S>,
    ) {
    }

    fn raw_decode(
        value: &Spanned<Literal, S>,
        _ctx: &mut knuffel::decode::Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        let l: &Literal = &value;
        Ok(match l {
            Literal::Bool(bool) => ETypeConst::Boolean(*bool),
            Literal::Int(num) => match TryInto::<u64>::try_into(num) {
                Ok(num) => ETypeConst::Scalar((num as ENumber).into()),
                Err(err) => {
                    return Err(DecodeError::Conversion {
                        span: value.span().clone(),
                        source: Box::new(err),
                    });
                }
            },
            Literal::Decimal(num) => match TryInto::<ENumber>::try_into(num) {
                Ok(num) => ETypeConst::Scalar((num).into()),
                Err(err) => {
                    return Err(DecodeError::Conversion {
                        span: value.span().clone(),
                        source: Box::new(err),
                    });
                }
            },
            Literal::String(str) => ETypeConst::String((**str).into()),
            Literal::Null => {
                return Err(DecodeError::Unsupported {
                    span: value.span().clone(),
                    message: Cow::Borrowed("Null constants are not supported"),
                })
            }
        })
    }
}
