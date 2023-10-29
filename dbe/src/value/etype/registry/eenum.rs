use std::fmt::{Display, Formatter};

use anyhow::{bail, Context};
use egui::Color32;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use ustr::{Ustr, UstrMap};

use crate::value::etype::registry::eitem::{EItemConst, EItemType, EItemTypeTrait};
use crate::value::etype::registry::{EObjectType, ETypeId, ETypesRegistry};
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

    pub(super) fn from_eitem(
        item: EItemType,
        name: String,
        registry: &mut ETypesRegistry,
    ) -> anyhow::Result<EEnumVariant> {
        let pat = match &item {
            EItemType::Number(_) => EnumPattern::Number,
            EItemType::String(_) => EnumPattern::String,
            EItemType::Boolean(_) => EnumPattern::Boolean,
            EItemType::Const(c) => EnumPattern::Const(c.value),
            EItemType::Generic(_) => EnumPattern::Const(ETypeConst::Null),
            EItemType::Enum(_) => {
                bail!("Enum variant can't be an enum")
            }
            EItemType::ObjectRef(id) => EnumPattern::Ref(id.ty),
            EItemType::ObjectId(_) => {
                bail!("Object Id can't appear as an Enum variant")
            }
            EItemType::Struct(s) => {
                registry.assert_defined(&s.id)?;
                let target_type = registry
                    .fetch_or_deserialize(s.id)
                    .context("Error during automatic pattern key detection\n> If you see recursion error at the top of this log, consider specifying `key` parameter manually")?;

                let data = match target_type {
                    EObjectType::Enum(_) => bail!("Enum variant can't be an another enum"),
                    EObjectType::Struct(data) => data,
                };
                let pat = if s.key.is_none() {
                    let pat = data.fields.iter().filter_map(|f| {
                        match &f.ty {
                            EItemType::Const(c) => {
                                Some((f.name, c.value))
                            }
                            _ => None,
                        }
                    }).exactly_one().map_err(|_| anyhow::anyhow!("Target struct `{}` contains multiple constant fields. Please specify pattern manually", s.id))?;

                    EnumPattern::StructField(pat.0.into(), pat.1)
                } else if let Some(key) = &s.key {
                    let field = data
                        .fields
                        .iter()
                        .find(|e| e.name == name)
                        .with_context(|| {
                            format!("Target struct `{}` doesn't contain a field `{}`", s.id, key,)
                        })?;

                    let EItemType::Const(c) = &field.ty else {
                        bail!(
                            "Target struct `{}` contains a field `{}` but it's not a constant",
                            s.id,
                            key,
                        )
                    };

                    EnumPattern::StructField(key.as_str().into(), c.value)
                } else {
                    bail!("Multiple pattern fields are not supported")
                };

                pat
            }
        };

        Ok(EEnumVariant::new(name, pat, item))
    }
}

pub type EEnumVariantWithId<'a> = (&'a EEnumVariant, &'a EEnumVariantId);

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

    pub fn apply_generics(
        mut self,
        arguments: &UstrMap<EItemType>,
        new_id: ETypeId,
        registry: &mut ETypesRegistry,
    ) -> anyhow::Result<Self> {
        self.ident = new_id;
        for variant in &mut self.variants {
            if let EItemType::Generic(g) = &variant.data {
                let item = arguments.get(&g.argument_name).with_context(|| {
                    format!("Generic argument `{}` is not provided", g.argument_name)
                })?;
                *variant = EEnumVariant::from_eitem(
                    item.clone(),
                    std::mem::take(&mut variant.name),
                    registry,
                )?;
            }
        }
        self.recalculate_variants();

        self.generic_arguments = vec![];

        Ok(self)
    }

    pub(super) fn add_variant(&mut self, variant: EEnumVariant) {
        self.variant_ids.push(EEnumVariantId {
            ident: self.ident,
            variant: variant.pat,
        });
        self.variants.push(variant);
    }

    fn recalculate_variants(&mut self) {
        self.variant_ids.truncate(self.variants.len());
        for (i, variant) in self.variants.iter().enumerate() {
            self.variant_ids[i] = EEnumVariantId {
                ident: self.ident,
                variant: variant.pat,
            }
        }
    }

    pub fn variants(&self) -> &Vec<EEnumVariant> {
        &self.variants
    }

    pub fn variant_ids(&self) -> &Vec<EEnumVariantId> {
        &self.variant_ids
    }

    pub fn variants_with_ids(&self) -> impl Iterator<Item = EEnumVariantWithId> {
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
