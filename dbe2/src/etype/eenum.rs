use crate::etype::eenum::variant::{EEnumVariant, EEnumVariantId, EEnumVariantWithId};
use crate::etype::eitem::EItemType;
use crate::json_utils::repr::Repr;
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use crate::value::EValue;
use miette::{bail, miette, Context};
use ustr::{Ustr, UstrMap};

pub mod pattern;
pub mod variant;

#[derive(Debug, Clone)]
pub struct EEnumData {
    pub generic_arguments: Vec<Ustr>,
    pub ident: ETypeId,
    pub repr: Option<Repr>,
    variants: Vec<EEnumVariant>,
    variant_ids: Vec<EEnumVariantId>,
}

impl EEnumData {
    pub fn new(ident: ETypeId, generic_arguments: Vec<Ustr>, repr: Option<Repr>) -> Self {
        Self {
            generic_arguments,
            ident,
            repr,
            variants: Default::default(),
            variant_ids: Default::default(),
        }
    }

    pub fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        let default_variant = self.variants.first().expect("Expect enum to not be empty");
        EValue::Enum {
            variant: EEnumVariantId {
                ident: self.ident,
                variant: default_variant.name,
            },
            data: Box::new(default_variant.default_value(registry)),
        }
    }

    pub fn apply_generics(
        mut self,
        arguments: &UstrMap<EItemType>,
        new_id: ETypeId,
        registry: &mut ETypesRegistry,
    ) -> miette::Result<Self> {
        self.ident = new_id;
        for variant in &mut self.variants {
            if let EItemType::Generic(g) = &variant.data {
                let item = arguments.get(&g.argument_name).ok_or_else(|| {
                    miette!("generic argument `{}` is not provided", g.argument_name)
                })?;
                *variant = EEnumVariant::from_eitem(
                    item.clone(),
                    std::mem::take(&mut variant.name),
                    registry,
                )?;
            }
        }
        self.recalculate_variants();

        // if let Ok((_, item)) = arguments.iter().exactly_one() {
        //     if self.color.is_none() {
        //         self.color = Some(item.ty().color(registry));
        //     }
        // }

        self.generic_arguments = vec![];

        Ok(self)
    }

    pub(crate) fn add_variant(&mut self, variant: EEnumVariant) {
        self.variant_ids.push(EEnumVariantId {
            ident: self.ident,
            variant: variant.name,
        });
        self.variants.push(variant);
    }

    fn recalculate_variants(&mut self) {
        self.variant_ids.truncate(self.variants.len());
        for (i, variant) in self.variants.iter().enumerate() {
            self.variant_ids[i] = EEnumVariantId {
                ident: self.ident,
                variant: variant.name,
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

    pub fn parse_json(&self, registry: &ETypesRegistry, data: JsonValue) -> miette::Result<EValue> {
        for variant in &self.variants {
            if variant.pat.matches_json(&data) {
                return variant
                    .data
                    .ty()
                    .parse_json(registry, data)
                    .with_context(|| format!("in enum variant {}", variant.name));
            }
        }

        bail!("value did not match any of enum variants")
    }
}
