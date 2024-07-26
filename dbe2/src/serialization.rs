use crate::etype::econst::ETypeConst;
use crate::etype::eenum::variant::EEnumVariant;
use crate::etype::eenum::EEnumData;
use crate::etype::estruct::{EStructData, EStructField};
use crate::json_utils::repr::Repr;
use crate::m_try;
use crate::registry::{EObjectType, ETypesRegistry};
use crate::serialization::item::ThingItem;
use crate::value::id::ETypeId;
use ahash::AHashMap;
use itertools::Itertools;
use knuffel::ast::{Literal, TypeName};
use knuffel::errors::DecodeError;
use knuffel::span::Spanned;
use knuffel::traits::ErrorSpan;
use knuffel::{DecodeScalar, Error};
use miette::{Context, IntoDiagnostic};
use ustr::Ustr;

mod item;

pub fn deserialize_etype(
    registry: &mut ETypesRegistry,
    id: ETypeId,
    data: &str,
) -> miette::Result<EObjectType> {
    let thing = parse_kdl(&id.to_string(), data)?;
    Ok(
        match thing
            .into_iter()
            .exactly_one()
            .into_diagnostic()
            .context("Can't define multiple things in one file")?
        {
            ThingVariant::Enum(value) => EObjectType::Enum(value.into_eenum(registry, id)?),
            ThingVariant::Struct(value) => EObjectType::Struct(value.into_estruct(registry, id)?),
        },
    )
}

fn parse_kdl(file_name: &str, data: &str) -> Result<Vec<ThingVariant>, Error> {
    knuffel::parse::<Vec<ThingVariant>>(file_name, data)
}

#[derive(Debug, knuffel::Decode)]
enum ThingVariant {
    Enum(ThingEnum),
    Struct(ThingStruct),
}

#[derive(Debug, knuffel::Decode)]
struct ThingStruct {
    #[knuffel(arguments, str)]
    pub generic_arguments: Vec<Ustr>,
    #[knuffel(property, str)]
    pub repr: Option<Repr>,
    #[knuffel(properties)]
    pub extra_properties: AHashMap<String, ETypeConst>,
    #[knuffel(children)]
    pub fields: Vec<ThingItem>,
}

#[derive(Debug, knuffel::Decode)]
struct ThingEnum {
    #[knuffel(arguments, str)]
    pub generic_arguments: Vec<Ustr>,
    #[knuffel(property, str)]
    pub repr: Option<Repr>,
    #[knuffel(properties)]
    pub extra_properties: AHashMap<String, ETypeConst>,
    #[knuffel(children)]
    variants: Vec<ThingItem>,
}

impl ThingStruct {
    fn into_estruct(
        self,
        registry: &mut ETypesRegistry,
        id: ETypeId,
    ) -> miette::Result<EStructData> {
        let mut data = EStructData::new(id, self.generic_arguments, self.repr);
        for e in self.fields {
            let field_name = e.name;
            m_try(|| {
                let (name, item) = e.into_item(registry, id, &data.generic_arguments)?;
                data.add_field(EStructField { name, ty: item })?;

                Ok(())
            })
            .with_context(|| format!("failed to initialize field {}", field_name))?;
        }

        Ok(data)
    }
}

impl ThingEnum {
    fn into_eenum(self, registry: &mut ETypesRegistry, id: ETypeId) -> miette::Result<EEnumData> {
        let mut data = EEnumData::new(id, self.generic_arguments, self.repr);
        for e in self.variants {
            let (name, item) = e.into_item(registry, id, &data.generic_arguments)?;
            data.add_variant(EEnumVariant::from_eitem(item, name, registry)?);
        }
        Ok(data)
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
        let l: &Literal = value;
        Ok(match l {
            Literal::Bool(bool) => ETypeConst::Boolean(*bool),
            Literal::Int(num) => match TryInto::<u64>::try_into(num) {
                Ok(num) => (num as f64).into(),
                Err(err) => {
                    return Err(DecodeError::Conversion {
                        span: value.span().clone(),
                        source: Err::<(), _>(err).into_diagnostic().err().unwrap().into(),
                    });
                }
            },
            Literal::Decimal(num) => match TryInto::<f64>::try_into(num) {
                Ok(num) => num.into(),
                Err(err) => {
                    return Err(DecodeError::Conversion {
                        span: value.span().clone(),
                        source: Err::<(), _>(err).into_diagnostic().err().unwrap().into(),
                    });
                }
            },
            Literal::String(str) => ETypeConst::String((**str).into()),
            Literal::Null => ETypeConst::Null,
        })
    }
}

mod tests {

    #[test]
    fn parse_test() {
        let ty = super::parse_kdl(
            "test.kdl",
            r#"
enum "Item" editor="enum" port="hollow" {
    generic "some" "Item"
    const "none" null
}
        "#,
        );

        match ty {
            Ok(data) => {
                let data = format!("{data:?}");
                println!("{data}")
            }
            Err(err) => {
                let err = miette::Report::from(err);
                println!("{err:?}");
                panic!("Fail")
            }
        }
    }
}
