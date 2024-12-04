use crate::etype::default::DefaultEValue;
use crate::etype::econst::ETypeConst;
use crate::etype::eenum::pattern::{EnumPattern, Tagged};
use crate::etype::eenum::EEnumData;
use crate::etype::eitem::EItemInfo;
use crate::etype::property::default_properties::PROP_FIELD_TAG;
use crate::etype::EDataType;
use crate::json_utils::repr::JsonRepr;
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use crate::value::EValue;
use miette::{bail, Context};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use ustr::Ustr;

#[derive(Debug, Clone)]
pub struct EEnumVariant {
    pub pat: EnumPattern,
    pub data: EItemInfo,
    pub name: Ustr,
}

impl EEnumVariant {
    pub fn default_value(&self, registry: &ETypesRegistry) -> DefaultEValue {
        self.data.default_value(registry)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn new(name: Ustr, pat: EnumPattern, data: EItemInfo) -> Self {
        Self { pat, data, name }
    }

    pub(crate) fn get_tag_value(&self) -> ETypeConst {
        PROP_FIELD_TAG.get(self.data.extra_properties(), ETypeConst::String(self.name))
    }

    pub(crate) fn from_eitem(
        item: EItemInfo,
        name: Ustr,
        registry: &mut ETypesRegistry,
        tagged_repr: Option<Tagged>,
        variant_name: Ustr,
    ) -> miette::Result<EEnumVariant> {
        if item.is_generic() {
            return Ok(EEnumVariant::new(name, EnumPattern::Never, item));
        }
        let pat = if let Some(repr) = tagged_repr {
            let tag = PROP_FIELD_TAG.get(item.extra_properties(), ETypeConst::String(variant_name));

            if repr.is_internal() && !item.ty().is_object() {
                bail!("internally tagged enums can only have object variants")
            }

            EnumPattern::Tagged { repr, tag }
        } else {
            match &item.ty() {
                EDataType::Boolean => EnumPattern::Boolean,
                EDataType::Number => EnumPattern::Number,
                EDataType::String => EnumPattern::String,
                EDataType::Const { value } => EnumPattern::Const(*value),
                EDataType::List { .. } => EnumPattern::List,
                EDataType::Map { .. } => EnumPattern::Map,
                EDataType::Object { ident } => {
                    let data = registry
                        .fetch_or_deserialize(*ident).context("This error might potentially be caused by the circular reference in types. Try specifying enum pattern manually")?;
                    // .ok_or_else(|| miette!("!!INTERNAL ERROR!! unknown object `{}` while deserializing enum pattern", ident))?;

                    if let Some(pat) = data.repr().and_then(|repr| repr.enum_pat()) {
                        pat
                    } else {
                        EnumPattern::UntaggedObject
                    }
                }
            }
        };

        Ok(EEnumVariant::new(name, pat, item))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
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
    pub fn variant_name(&self) -> Ustr {
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
        self.variant(registry)
            .map(|e| e.default_value(registry).into_owned())
    }
}

impl Display for EEnumVariantId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.ident, self.variant)
    }
}

pub type EEnumVariantWithId<'a> = (&'a EEnumVariant, &'a EEnumVariantId);
