use crate::etype::econst::ETypeConst;
use crate::etype::eitem::{EItemType, EItemTypeGeneric, EItemTypeSpecific};
use crate::etype::EDataType;
use crate::m_try;
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use ahash::AHashMap;
use itertools::Itertools;
use miette::{bail, Context};
use strum::EnumString;

use ustr::{Ustr, UstrMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString)]
pub enum ThingItemKind {
    Boolean,
    Number,
    String,
    Id,
    Ref,
    Enum,
    Struct,
    Const,
    List,
    Map,
    Generic,
}

#[derive(Debug, knuffel::Decode)]
pub struct ThingItem {
    #[knuffel(node_name)]
    pub kind: ThingItemKind,
    #[knuffel(argument, str)]
    pub name: Ustr,
    #[knuffel(arguments)]
    pub arguments: Vec<ETypeConst>,
    #[knuffel(properties)]
    pub extra_properties: AHashMap<String, ETypeConst>,
    #[knuffel(children)]
    pub generics: Vec<ThingItem>,
}

impl ThingItem {
    pub fn into_item(self, registry: &mut ETypesRegistry) -> miette::Result<(Ustr, EItemType)> {
        let no_args = || {
            if !self.arguments.is_empty() {
                bail!(
                    "Expected no arguments, but got {} arguments instead",
                    self.arguments.len()
                );
            }
            Ok(())
        };
        let generics_len = self.generics.len();
        let no_generics = || {
            if generics_len > 0 {
                bail!(
                    "Expected no generic children, but got {} generic children instead",
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
                    if arg.extra_properties.len() > 0 {
                        bail!(
                            "Generic arguments can't contain extra properties, but got {}",
                            arg.extra_properties
                                .into_keys()
                                .map(|k| format!("`{k}`"))
                                .join(", ")
                        )
                    }
                    arg.into_item(registry)
                })
                .with_context(|| {
                    format!("While parsing generic child at position {i} with name {name}")
                })?;
                items.insert(k, v);
            }

            miette::Result::<UstrMap<EItemType>>::Ok(items)
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
            ThingItemKind::Id => {
                let [ty] = expect_args(self.arguments)?;
                let ty = id(ty, 0)?;
                registry.assert_defined(&ty)?;
                no_generics()?;
                EDataType::Id { ty }
            }
            ThingItemKind::Ref => {
                let [ty] = expect_args(self.arguments)?;
                let ty = id(ty, 0)?;
                registry.assert_defined(&ty)?;
                no_generics()?;
                EDataType::Ref { ty }
            }
            ThingItemKind::Struct => {
                let [ty] = expect_args(self.arguments)?;
                let mut ty = id(ty, 0)?;
                registry.assert_defined(&ty)?;
                let generics = generics(registry)?;
                if !generics.is_empty() {
                    ty = registry.make_generic(ty, generics)?;
                }
                EDataType::Id { ty }
            }
            ThingItemKind::Enum => {
                let [ty] = expect_args(self.arguments)?;
                let mut ty = id(ty, 0)?;
                registry.assert_defined(&ty)?;
                let generics = generics(registry)?;
                if !generics.is_empty() {
                    ty = registry.make_generic(ty, generics)?;
                }
                EDataType::Id { ty }
            }
            ThingItemKind::Const => {
                let [value] = expect_args(self.arguments)?;
                EDataType::Const { value }
            }
            ThingItemKind::List => {
                no_args()?;
                let generics = generics(registry)?;
                let Some(ty) = generics.get(&Ustr::from("Item")) else {
                    bail!("Generic argument `Item` is not provided");
                };

                registry.register_list(ty.ty())
            }
            ThingItemKind::Map => {
                no_args()?;
                let generics = generics(registry)?;
                let Some(key) = generics.get(&Ustr::from("Key")) else {
                    bail!("Generic argument `Key` is not provided");
                };
                let Some(value) = generics.get(&Ustr::from("Item")) else {
                    bail!("Generic argument `Item` is not provided");
                };

                registry.register_map(key.ty(), value.ty())
            }
            ThingItemKind::Generic => {
                return Ok((
                    self.name,
                    EItemType::Generic(EItemTypeGeneric {
                        argument_name: self.name,
                        extra_properties: self.extra_properties,
                    }),
                ))
            }
        };

        Ok((
            self.name,
            EItemType::Specific(EItemTypeSpecific {
                ty,
                extra_properties: self.extra_properties,
            }),
        ))
    }
}

fn expect_args<const N: usize, T>(args: Vec<T>) -> miette::Result<[T; N]> {
    if args.len() != N {
        bail!(
            "Expected {} arguments, but got {} arguments instead",
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
            "Argument at position {pos} is expected to be an item ID, but got {} instead",
            pos
        )
    };
    ETypeId::parse(&str)
}
