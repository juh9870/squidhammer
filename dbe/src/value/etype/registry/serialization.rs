use anyhow::{anyhow, Context};
use itertools::Itertools;
use knuffel::ast::{Literal, TypeName};
use knuffel::errors::DecodeError;
use knuffel::span::Spanned;
use knuffel::traits::ErrorSpan;
use knuffel::DecodeScalar;
use miette::{GraphicalReportHandler, GraphicalTheme};

use ustr::Ustr;

use crate::value::etype::registry::eenum::EEnumData;
use crate::value::etype::registry::estruct::EStructData;
use crate::value::etype::registry::serialization::field::{ThingFieldTrait, ThingItem};
use crate::value::etype::registry::{EObjectType, ETypeId, ETypesRegistry};
use crate::value::etype::ETypeConst;
use crate::value::ENumber;

mod field;
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
    id: ETypeId,
    data: &str,
) -> Result<EObjectType, anyhow::Error> {
    let thing = knuffel::parse::<Vec<ThingVariant>>(&id.to_string(), data).map_err(|err| {
        let mut report = String::new();
        if let Err(_) = GraphicalReportHandler::new()
            .with_theme(GraphicalTheme::none())
            .render_report(&mut report, &err)
        {
            panic!("Failed to format error");
        }
        anyhow!("{report}")
    })?;
    Ok(
        match thing
            .into_iter()
            .exactly_one()
            .context("Can't define multiple things in one file")?
        {
            ThingVariant::Enum(value) => EObjectType::Enum(value.into_eenum(registry, id)?),
            ThingVariant::Struct(value) => EObjectType::Struct(value.into_estruct(registry, id)?),
        },
    )
}

// #[derive(Debug, knuffel::Decode)]
// struct ThingTop {
//     #[knuffel(child)]
//     value: ThingVariant,
// }

#[derive(Debug, knuffel::Decode)]
enum ThingVariant {
    Enum(ThingEnum),
    Struct(ThingStruct),
}

#[derive(Debug, knuffel::Decode)]
struct ThingStruct {
    #[knuffel(arguments, str)]
    pub generic_arguments: Vec<Ustr>,
    #[knuffel(children)]
    pub fields: Vec<ThingItem>,
}

impl ThingStruct {
    fn into_estruct(
        self,
        registry: &mut ETypesRegistry,
        id: ETypeId,
    ) -> anyhow::Result<EStructData> {
        let mut data = EStructData::new(id);
        data.generic_arguments = self.generic_arguments;
        for x in self.fields {
            let path = format!("{id}:{}", x.name());
            data.add_field(x.into_struct_field(registry, id, &path)?)?;
        }

        Ok(data)
    }
}

#[derive(Debug, knuffel::Decode)]
struct ThingEnum {
    #[knuffel(arguments, str)]
    pub generic_arguments: Vec<Ustr>,
    #[knuffel(children)]
    variants: Vec<ThingItem>,
}

impl ThingEnum {
    fn into_eenum(self, registry: &mut ETypesRegistry, id: ETypeId) -> anyhow::Result<EEnumData> {
        Ok(EEnumData {
            ident: id,
            generic_arguments: self.generic_arguments,
            variants: self
                .variants
                .into_iter()
                .map(|e| {
                    let path = format!("{id}::{}", e.name());
                    e.into_enum_variant(registry, id, &path)
                })
                .try_collect()?,
        })
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
                Ok(num) => ETypeConst::Number((num as ENumber).into()),
                Err(err) => {
                    return Err(DecodeError::Conversion {
                        span: value.span().clone(),
                        source: Box::new(err),
                    });
                }
            },
            Literal::Decimal(num) => match TryInto::<ENumber>::try_into(num) {
                Ok(num) => ETypeConst::Number((num).into()),
                Err(err) => {
                    return Err(DecodeError::Conversion {
                        span: value.span().clone(),
                        source: Box::new(err),
                    });
                }
            },
            Literal::String(str) => ETypeConst::String((**str).into()),
            Literal::Null => ETypeConst::Null,
        })
    }
}
