use crate::etype::econst::ETypeConst;
use crate::etype::eitem::{EItemInfo, EItemInfoGeneric, EItemInfoSpecific};
use crate::etype::property::field_props;
use crate::etype::EDataType;
use crate::m_try;
use crate::registry::ETypesRegistry;
use crate::serialization::validators;
use crate::value::id::ETypeId;
use ahash::AHashMap;
use itertools::Itertools;
use miette::{bail, Context, Diagnostic};
use std::fmt::Display;
use std::sync::Arc;
use strum::EnumString;
use thiserror::Error;
use ustr::{Ustr, UstrMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum ThingItemKind {
    Boolean,
    Number,
    String,
    Object,
    Const,
    List,
    Map,
    Generic,
}

#[derive(Debug, knus::Decode)]
pub struct ThingItem {
    #[knus(node_name)]
    pub kind: ThingItemKind,
    #[knus(argument, str)]
    pub name: Ustr,
    #[knus(arguments)]
    pub arguments: Vec<ETypeConst>,
    #[knus(properties)]
    pub extra_properties: AHashMap<String, ETypeConst>,
    #[knus(children)]
    pub generics: Vec<ThingItem>,
}

impl ThingItem {
    pub fn into_item(
        self,
        registry: &mut ETypesRegistry,
        generic_arguments: &[Ustr],
    ) -> miette::Result<(Ustr, EItemInfo)> {
        let no_args = || {
            if !self.arguments.is_empty() {
                bail!(
                    "expected no arguments, but got {} arguments instead",
                    self.arguments.len()
                );
            }
            Ok(())
        };
        let generics_len = self.generics.len();
        let no_generics = || {
            if generics_len > 0 {
                bail!(
                    "expected no generic children, but got {} generic children instead",
                    generics_len
                );
            }
            Ok(())
        };
        let generics = |registry: &mut ETypesRegistry| {
            let mut items = UstrMap::default();
            for (i, arg) in self.generics.into_iter().enumerate() {
                let name = arg.name;
                let (k, v) = m_try(|| {
                    // if arg.extra_properties.len() > 0 {
                    //     bail!(
                    //         "generic arguments can't contain extra properties, but got {}",
                    //         arg.extra_properties
                    //             .into_keys()
                    //             .map(|k| format!("`{k}`"))
                    //             .join(", ")
                    //     )
                    // }
                    arg.into_item(registry, generic_arguments)
                })
                .with_context(|| {
                    format!("failed to parse generic child at position {i} with name {name}")
                })?;
                items.insert(k, v);
            }

            miette::Result::<UstrMap<EItemInfo>>::Ok(items)
        };

        let ty = match self.kind {
            ThingItemKind::Boolean => {
                no_args()?;
                no_generics()?;
                EDataType::Boolean
            }
            ThingItemKind::Number => {
                no_args()?;
                no_generics()?;
                EDataType::Number
            }
            ThingItemKind::String => {
                no_args()?;
                no_generics()?;
                EDataType::String
            }
            ThingItemKind::Object => {
                let [ty] = expect_args(self.arguments)?;
                let mut ty = id(ty, 0)?;
                registry.assert_defined(&ty)?;
                let generics = generics(registry)?;
                if !generics.is_empty() {
                    ty = registry.make_generic(ty, generics)?;
                }
                EDataType::Object { ident: ty }
            }
            ThingItemKind::Const => {
                let [value] = expect_args(self.arguments)?;
                EDataType::Const { value }
            }
            ThingItemKind::List => {
                no_args()?;
                let generics = generics(registry)?;
                let Some(ty) = generics.get(&Ustr::from("Item")) else {
                    bail!("generic argument `Item` is not provided");
                };

                registry.list_of(ty.ty())
            }
            ThingItemKind::Map => {
                no_args()?;
                let generics = generics(registry)?;
                let Some(key) = generics.get(&Ustr::from("Key")) else {
                    bail!("generic argument `Key` is not provided");
                };
                let Some(value) = generics.get(&Ustr::from("Item")) else {
                    bail!("generic argument `Item` is not provided");
                };

                registry.map_of(key.ty(), value.ty())
            }
            ThingItemKind::Generic => {
                let [arg] = expect_args(self.arguments)?;
                let arg = generic_name(arg, 0, generic_arguments)?;
                return Ok((
                    self.name,
                    EItemInfo::Generic(Arc::new(EItemInfoGeneric {
                        argument_name: arg,
                        extra_properties: field_props(self.extra_properties)?,
                        validators: vec![],
                    })),
                ));
            }
        };

        let validators = validators(&self.extra_properties)?;

        Ok((
            self.name,
            EItemInfo::Specific(Arc::new(EItemInfoSpecific {
                ty,
                extra_properties: field_props(self.extra_properties)?,
                validators,
            })),
        ))
    }
}

fn expect_args<const N: usize, T>(args: Vec<T>) -> miette::Result<[T; N]> {
    if args.len() != N {
        bail!(
            "expected {} arguments, but got {} arguments instead",
            N,
            args.len()
        );
    }
    Ok(args
        .try_into()
        .unwrap_or_else(|_| unreachable!("Length was checked before")))
}

fn id(val: ETypeConst, pos: usize) -> miette::Result<ETypeId> {
    let ETypeConst::String(str) = val else {
        bail!(
            "argument at position {pos} is expected to be an item ID, but got {} instead",
            pos
        )
    };
    ETypeId::parse(&str)
}

fn generic_name(val: ETypeConst, pos: usize, generic_arguments: &[Ustr]) -> miette::Result<Ustr> {
    let ETypeConst::String(str) = val else {
        bail!(
            "argument at position {pos} is expected to be a generic type name, but got {} instead",
            pos
        )
    };

    if !generic_arguments.contains(&str) {
        return Err(BadGenericArg(str, pos, generic_arguments.to_vec()).into());
    }
    Ok(str)
}

#[derive(Debug, Error)]
#[error("argument at position {} has generic type `{}` which is not defined in the object's generic arguments", .1, .0)]
struct BadGenericArg(Ustr, usize, Vec<Ustr>);

impl Diagnostic for BadGenericArg {
    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        Some(Box::new(format!(
            "Expected one of the following generic arguments: {}",
            self.2.iter().join(", ")
        )))
    }
}
