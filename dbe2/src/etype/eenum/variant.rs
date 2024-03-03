use crate::etype::econst::ETypeConst;
use crate::etype::eenum::pattern::EnumPattern;
use crate::etype::eenum::EEnumData;
use crate::etype::eitem::EItemType;
use crate::etype::EDataType;
use crate::registry::{EObjectType, ETypesRegistry};
use crate::value::id::ETypeId;
use crate::value::EValue;
use itertools::Itertools;
use miette::{bail, miette, Context};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use ustr::Ustr;

#[derive(Debug, Clone)]
pub struct EEnumVariant {
    pub pat: EnumPattern,
    pub data: EItemType,
    pub name: Ustr,
}

impl EEnumVariant {
    pub fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        self.data.default_value(registry)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn new(name: Ustr, pat: EnumPattern, data: EItemType) -> Self {
        Self { pat, data, name }
    }

    pub(crate) fn from_eitem(
        item: EItemType,
        name: Ustr,
        registry: &mut ETypesRegistry,
    ) -> miette::Result<EEnumVariant> {
        let pat = match &item.ty() {
            EDataType::Boolean => EnumPattern::Boolean,
            EDataType::Number => EnumPattern::Number,
            EDataType::String => EnumPattern::String,
            EDataType::Id { .. } => {
                bail!("Object Id can't appear as an Enum variant");
            }
            EDataType::Ref { ty } => EnumPattern::Ref(*ty),
            EDataType::Const { value } => EnumPattern::Const(*value),
            EDataType::List { .. } => EnumPattern::List,
            EDataType::Map { .. } => EnumPattern::Map,
            EDataType::Object { ident } => {
                registry.assert_defined(ident)?;

                let target_type = registry.fetch_or_deserialize(*ident).context("Error during automatic pattern key detection\n> If you see recursion error at the top of this log, consider specifying `key` parameter manually")?;
                let data = match target_type {
                    EObjectType::Enum(_) => bail!("Enum variant can't be an another enum"),
                    EObjectType::Struct(data) => data,
                };

                let pat = if !item.extra_properties().contains_key("key") {
                    let pat = data.fields.iter().filter_map(|f| {
                        match &f.ty.ty() {
                            EDataType::Const{value} => {
                                Some((f.name, *value))
                            }
                            _ => None,
                        }
                    }).exactly_one().map_err(|_| miette!("Target struct `{}` contains multiple constant fields. Please specify pattern manually", ident))?;

                    EnumPattern::StructField(pat.0, pat.1)
                } else if let Some(key) = item.extra_properties().get("key") {
                    let ETypeConst::String(key) = key else {
                        bail!(
                            "Type of the `key` field must be string. Instead got {}",
                            key
                        );
                    };
                    let field = data.fields.iter().find(|e| e.name == name).ok_or_else(|| {
                        miette!(
                            "Target struct `{}` doesn't contain a field `{}`",
                            ident,
                            key,
                        )
                    })?;

                    let EDataType::Const { value } = field.ty.ty() else {
                        bail!(
                            "Target struct `{}` contains a field `{}` but it's not a constant",
                            ident,
                            key,
                        )
                    };

                    EnumPattern::StructField(key.as_str().into(), value)
                } else {
                    bail!("Multiple pattern fields are not supported")
                };

                pat
            }
        };

        Ok(EEnumVariant::new(name, pat, item))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct EEnumVariantId {
    pub(super) ident: ETypeId,
    pub(super) variant: Ustr,
}

impl EEnumVariantId {
    pub fn enum_id(&self) -> ETypeId {
        self.ident
    }
    pub fn matches(&self, variant: &EEnumVariant) -> bool {
        self.variant == variant.name
    }
    pub fn pattern(&self) -> Ustr {
        self.variant
    }

    pub fn enum_variant<'a>(
        &self,
        registry: &'a ETypesRegistry,
    ) -> Option<(&'a EEnumData, &'a EEnumVariant)> {
        let eenum = registry.get_enum(&self.ident)?;
        let variant = eenum.variants.iter().find(|v| self.variant == v.name)?;
        Some((eenum, variant))
    }

    pub fn variant<'a>(&self, registry: &'a ETypesRegistry) -> Option<&'a EEnumVariant> {
        self.enum_variant(registry).map(|e| e.1)
    }

    pub fn default_value(&self, registry: &ETypesRegistry) -> Option<EValue> {
        self.variant(registry).map(|e| e.default_value(registry))
    }
}

impl Display for EEnumVariantId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.ident, self.variant)
    }
}

pub type EEnumVariantWithId<'a> = (&'a EEnumVariant, &'a EEnumVariantId);
