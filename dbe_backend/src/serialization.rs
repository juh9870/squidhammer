use crate::etype::econst::ETypeConst;
use crate::etype::eenum::pattern::Tagged;
use crate::etype::eenum::variant::EEnumVariant;
use crate::etype::eenum::EEnumData;
use crate::etype::estruct::{EStructData, EStructField};
use crate::etype::property::object_props;
use crate::json_utils::repr::Repr;
use crate::m_try;
use crate::registry::{EObjectType, ETypesRegistry};
use crate::serialization::item::ThingItem;
use crate::validation::{validator_by_name, Validator};
use crate::value::id::ETypeId;
use ahash::AHashMap;
use itertools::Itertools;
use knus::ast::{Literal, TypeName};
use knus::errors::DecodeError;
use knus::span::Spanned;
use knus::traits::ErrorSpan;
use knus::{DecodeScalar, Error};
use miette::{bail, miette, Context, IntoDiagnostic};
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
    knus::parse::<Vec<ThingVariant>>(file_name, data)
}

#[derive(Debug, knus::Decode)]
enum ThingVariant {
    Enum(ThingEnum),
    Struct(ThingStruct),
}

#[derive(Debug, knus::Decode)]
struct ThingStruct {
    #[knus(arguments, str)]
    pub generic_arguments: Vec<Ustr>,
    #[knus(property, str)]
    pub repr: Option<Repr>,
    #[knus(properties)]
    pub extra_properties: AHashMap<String, ETypeConst>,
    #[knus(children)]
    pub fields: Vec<ThingItem>,
}

#[derive(Debug, knus::Decode)]
struct ThingEnum {
    #[knus(arguments, str)]
    pub generic_arguments: Vec<Ustr>,
    #[knus(property, str)]
    pub repr: Option<Repr>,
    #[knus(property, str)]
    pub tag: Option<Ustr>,
    #[knus(property, str)]
    pub content: Option<Ustr>,
    #[knus(properties)]
    pub extra_properties: AHashMap<String, ETypeConst>,
    #[knus(children)]
    variants: Vec<ThingItem>,
}

impl ThingStruct {
    fn into_estruct(
        self,
        registry: &mut ETypesRegistry,
        id: ETypeId,
    ) -> miette::Result<EStructData> {
        let mut data = EStructData::new(
            id,
            self.generic_arguments,
            self.repr,
            object_props(self.extra_properties)?,
        );
        for e in self.fields {
            let field_name = e.name;
            m_try(|| {
                let (name, item) = e.into_item(registry, &data.generic_arguments)?;
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
        let repr = if let Some(tag) = self.tag {
            if tag.as_str() == "{}" {
                if self.content.is_some() {
                    bail!("`content` field can't be used on externally tagged enum")
                }
                Some(Tagged::External)
            } else if let Some(content) = self.content {
                Some(Tagged::Adjacent {
                    tag_field: tag,
                    content_field: content,
                })
            } else {
                Some(Tagged::Internal { tag_field: tag })
            }
        } else {
            if self.content.is_some() {
                bail!("`content` field can't be used on untagged enum")
            }
            None
        };

        let mut data = EEnumData::new(
            id,
            self.generic_arguments,
            self.repr,
            repr,
            object_props(self.extra_properties)?,
        );
        for e in self.variants {
            let (name, item) = e.into_item(registry, &data.generic_arguments)?;
            data.add_variant(EEnumVariant::from_eitem(item, name, registry, repr, name)?);
        }
        Ok(data)
    }
}

fn validators(extra_properties: &AHashMap<String, ETypeConst>) -> miette::Result<Vec<Validator>> {
    let mut validators = vec![];
    for (key, optional) in [("validator", false), ("editor", true)] {
        if let Some(prop) = extra_properties.get(key) {
            let editor_name = prop
                .as_string()
                .ok_or_else(|| miette!("property `{key}` is expected to be a string"))?;

            let Some(validator) = validator_by_name(editor_name) else {
                if optional {
                    continue;
                }
                bail!("unknown validator `{editor_name}`")
            };

            validators.push(validator);
        }
    }
    Ok(validators)
}

impl<S: ErrorSpan> DecodeScalar<S> for ETypeConst {
    fn type_check(_type_name: &Option<Spanned<TypeName, S>>, _ctx: &mut knus::decode::Context<S>) {}

    fn raw_decode(
        value: &Spanned<Literal, S>,
        _ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        let l: &Literal = value;
        Ok(match l {
            Literal::Bool(bool) => ETypeConst::Boolean(*bool),
            Literal::Int(num) => match TryInto::<i64>::try_into(num) {
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
