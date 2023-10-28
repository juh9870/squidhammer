use std::fmt::{Display, Formatter};

use anyhow::Context;
use egui::Color32;
use serde::{Deserialize, Serialize};
use ustr::{Ustr, UstrMap};

use crate::value::etype::registry::eitem::{EItemConst, EItemType, EItemTypeTrait};
use crate::value::etype::registry::{ETypeId, ETypesRegistry};
use crate::value::etype::ETypeConst;
use crate::value::EValue;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum EnumPattern {
    StructField(Ustr, ETypeConst),
    Boolean,
    Number,
    String,
    Ref(ETypeId),
    Const(ETypeConst),
}
impl Display for EnumPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EnumPattern::StructField(field, ty) => write!(f, "{{\"{field}\": \"{ty}\"}}"),
            EnumPattern::Boolean => write!(f, "{{boolean}}"),
            EnumPattern::Number => write!(f, "{{number}}"),
            EnumPattern::String => write!(f, "{{string}}"),
            EnumPattern::Const(ty) => write!(f, "{{{ty}}}"),
            EnumPattern::Ref(ty) => write!(f, "{{Ref<{ty}>}}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EEnumVariant {
    pub pat: EnumPattern,
    pub data: EItemType,
    pub name: String,
}

impl EEnumVariant {
    pub fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        self.data.default_value(registry)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn null() -> EEnumVariant {
        EEnumVariant {
            pat: EnumPattern::Const(ETypeConst::Null),
            data: EItemType::Const(EItemConst {
                value: ETypeConst::Null,
            }),
            name: "null".to_string(),
        }
    }

    pub(super) fn new(name: String, pat: EnumPattern, data: EItemType) -> Self {
        Self { pat, data, name }
    }
}

#[derive(Debug, Clone)]
pub struct EEnumData {
    pub generic_arguments: Vec<Ustr>,
    pub ident: ETypeId,
    variants: Vec<EEnumVariant>,
    variant_ids: Vec<EEnumVariantId>,
    pub default_editor: Option<String>,
    pub color: Option<Color32>,
}

impl EEnumData {
    pub fn new(
        ident: ETypeId,
        generic_arguments: Vec<Ustr>,
        default_editor: Option<String>,
        color: Option<Color32>,
    ) -> Self {
        Self {
            generic_arguments,
            ident,
            variants: Default::default(),
            variant_ids: Default::default(),
            default_editor,
            color,
        }
    }
    pub fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        let default_variant = self.variants.first().expect("Expect enum to not be empty");
        EValue::Enum {
            variant: EEnumVariantId {
                ident: self.ident,
                variant: default_variant.pat,
            },
            data: Box::new(default_variant.default_value(registry)),
        }
    }

    pub fn clone_with_ident(&self, ident: ETypeId) -> EEnumData {
        let mut clone = self.clone();
        clone.ident = ident;
        for variant in &mut clone.variant_ids {
            variant.ident = ident
        }
        clone
    }

    pub fn apply_generics(
        &self,
        arguments: &UstrMap<EItemType>,
        new_id: ETypeId,
    ) -> anyhow::Result<Self> {
        let mut cloned = self.clone_with_ident(new_id);
        for x in &mut cloned.variants {
            if let EItemType::Generic(g) = &x.data {
                let item = arguments.get(&g.argument_name).with_context(|| {
                    format!("Generic argument `{}` is not provided", g.argument_name)
                })?;
                x.data = item.clone();
            }
        }

        cloned.generic_arguments = vec![];

        Ok(cloned)
    }

    pub(super) fn add_variant(&mut self, variant: EEnumVariant) {
        self.variant_ids.push(EEnumVariantId {
            ident: self.ident,
            variant: variant.pat,
        });
        self.variants.push(variant);
    }

    pub fn variants(&self) -> &Vec<EEnumVariant> {
        &self.variants
    }

    pub fn variant_ids(&self) -> &Vec<EEnumVariantId> {
        &self.variant_ids
    }

    pub fn variants_with_ids(&self) -> impl Iterator<Item = (&EEnumVariant, &EEnumVariantId)> {
        self.variants.iter().zip(self.variant_ids.iter())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct EEnumVariantId {
    ident: ETypeId,
    // Data types are currently unique
    variant: EnumPattern,
}

impl EEnumVariantId {
    pub fn enum_id(&self) -> ETypeId {
        self.ident
    }
    pub fn matches(&self, variant: &EEnumVariant) -> bool {
        return self.variant == variant.pat;
    }
    pub fn pattern(&self) -> EnumPattern {
        self.variant
    }

    pub fn enum_variant<'a>(
        &self,
        registry: &'a ETypesRegistry,
    ) -> Option<(&'a EEnumData, &'a EEnumVariant)> {
        let eenum = registry.get_enum(&self.ident)?;
        let variant = eenum.variants.iter().find(|v| v.pat == self.variant)?;
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
