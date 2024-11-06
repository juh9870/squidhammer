use crate::etype::econst::ETypeConst;
use crate::etype::eenum::pattern::{EnumPattern, Tagged};
use crate::etype::eenum::variant::{EEnumVariant, EEnumVariantId, EEnumVariantWithId};
use crate::etype::eitem::EItemInfo;
use crate::json_utils::repr::{JsonRepr, Repr};
use crate::json_utils::{json_kind, JsonMap, JsonValue};
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use crate::value::EValue;
use ahash::AHashMap;
use itertools::Itertools;
use miette::{bail, miette, Context};
use ustr::{Ustr, UstrMap};

pub mod pattern;
pub mod variant;

#[derive(Debug, Clone)]
pub struct EEnumData {
    pub generic_arguments: Vec<Ustr>,
    pub generic_arguments_values: Vec<EItemInfo>,
    pub ident: ETypeId,
    pub repr: Option<Repr>,
    pub extra_properties: AHashMap<String, ETypeConst>,
    tagged_repr: Option<Tagged>,
    variants: Vec<EEnumVariant>,
    variant_ids: Vec<EEnumVariantId>,
}

impl EEnumData {
    pub fn new(
        ident: ETypeId,
        generic_arguments: Vec<Ustr>,
        repr: Option<Repr>,
        tagged_repr: Option<Tagged>,
        extra_properties: AHashMap<String, ETypeConst>,
    ) -> Self {
        Self {
            generic_arguments,
            generic_arguments_values: vec![],
            ident,
            repr,
            extra_properties,
            tagged_repr,
            variants: Default::default(),
            variant_ids: Default::default(),
        }
    }

    pub(crate) fn default_value_inner(&self, registry: &ETypesRegistry) -> EValue {
        let default_variant = self.variants.first().expect("Expect enum to not be empty");
        EValue::Enum {
            variant: EEnumVariantId {
                ident: self.ident,
                variant: default_variant.name,
            },
            data: Box::new(default_variant.default_value(registry).into_owned()),
        }
    }

    pub fn apply_generics(
        mut self,
        arguments: &UstrMap<EItemInfo>,
        new_id: ETypeId,
        registry: &mut ETypesRegistry,
    ) -> miette::Result<Self> {
        self.ident = new_id;
        for variant in &mut self.variants {
            if let EItemInfo::Generic(g) = &variant.data {
                let item = arguments.get(&g.argument_name).ok_or_else(|| {
                    miette!("generic argument `{}` is not provided", g.argument_name)
                })?;
                *variant = EEnumVariant::from_eitem(
                    item.clone(),
                    std::mem::take(&mut variant.name),
                    registry,
                    self.tagged_repr,
                    variant.name,
                )?;
            }
        }
        self.recalculate_variants();

        for arg in &self.generic_arguments {
            let item = arguments
                .get(arg)
                .ok_or_else(|| miette!("generic argument `{}` is not provided", arg))?;
            self.generic_arguments_values.push(item.clone());
        }

        // if let Ok((_, item)) = arguments.iter().exactly_one() {
        //     if self.color.is_none() {
        //         self.color = Some(item.ty().color(registry));
        //     }
        // }

        // self.generic_arguments = vec![];

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

    pub(crate) fn parse_json(
        &self,
        registry: &ETypesRegistry,
        data: &mut JsonValue,
        inline: bool,
    ) -> miette::Result<EValue> {
        if let Some(repr) = self.tagged_repr {
            let JsonValue::Object(data) = &data else {
                bail!(
                    "tagged enum pattern matched against non-object json data: {}",
                    json_kind(data)
                )
            };
            match repr {
                Tagged::External => {
                    if !inline && data.len() > 1 {
                        bail!("more than one field is detected in externally tagged field")
                    } else if data.is_empty() {
                        bail!("value of externally tagged enum can not be an empty object")
                    }
                }
                Tagged::Internal { tag_field: _ } => {
                    // if !data.contains_key(tag_field.as_str()) {
                    //     bail!("tag field `{tag_field}` is missing in internally tagged enum")
                    // }
                }
                Tagged::Adjacent {
                    tag_field,
                    content_field,
                } => {
                    // if !data.contains_key(tag_field.as_str()) {
                    //     bail!("tag field `{tag_field}` is missing in adjacently tagged enum")
                    // }
                    if !inline {
                        let mut unknown_keys = data
                            .keys()
                            .filter(|key| {
                                key.as_str() != tag_field.as_str()
                                    && key.as_str() != content_field.as_str()
                            })
                            .peekable();

                        if unknown_keys.peek().is_some() {
                            bail!(
                                "adjacently tagged enum contains unknown fields: {}",
                                unknown_keys.map(|k| format!("`{k}`")).join(", ")
                            )
                        }
                    }
                }
            }
        }
        for (variant, id) in self.variants_with_ids() {
            if variant.pat.matches_json(data) {
                let mut data_holder: Option<JsonValue> = None;
                let data = if let EnumPattern::Tagged { repr, tag } = &variant.pat {
                    let JsonValue::Object(fields) = data else {
                        bail!(
                            "tagged enum pattern matched against non-object json data: {}",
                            json_kind(data)
                        )
                    };
                    match repr {
                        Tagged::External => data_holder.insert(fields
                            .remove(tag.as_json_key().as_str())
                            .ok_or_else(|| miette!("!!INTERNAL ERROR!! externally tagged enum variant lacks the tag field `{tag}`, even though the pattern matched"))?),
                        Tagged::Internal { tag_field } => {
                            fields.remove(tag_field.as_str());
                            data
                        }
                        Tagged::Adjacent { content_field, .. } =>
                            data_holder.insert(fields.remove(content_field.as_str())
                                .ok_or_else(|| miette!("Adjacently tagged enum variant lacks the content field `{tag}`"))?)
                    }
                } else {
                    data
                };

                let content = variant
                    .data
                    .ty()
                    .parse_json(registry, data, false)
                    .with_context(|| format!("in enum variant {}", variant.name))?;

                return Ok(EValue::Enum {
                    variant: *id,
                    data: Box::new(content),
                });
            }
        }

        bail!("value did not match any of enum variants")
    }

    pub(crate) fn write_json(
        &self,
        registry: &ETypesRegistry,
        value: &EValue,
        variant: &EEnumVariantId,
    ) -> miette::Result<JsonValue> {
        let variant = self
            .variants
            .iter()
            .find(|v| v.name == variant.variant)
            .ok_or_else(|| {
                miette!(
                    "enum variant `{}` not found in enum `{}`",
                    variant.variant,
                    self.ident
                )
            })?;
        let mut json_content = value.write_json(registry).context("in enum value")?;
        let json_value = if let Some(repr) = self.tagged_repr {
            let tag = variant.get_tag_value();
            match repr {
                Tagged::External => {
                    let mut obj = JsonMap::new();
                    obj.insert(tag.as_json_key().to_string(), json_content);
                    JsonValue::Object(obj)
                }
                Tagged::Internal { tag_field } => {
                    let JsonValue::Object(obj) = &mut json_content else {
                        bail!("internally tagged enum variant should serialize to object, but instead got {}", json_content)
                    };

                    if obj.contains_key(tag_field.as_str()) {
                        bail!("tag field `{tag_field}` is already present in serialized value of internally tagged enum variant")
                    }

                    obj.insert(tag_field.to_string(), tag.as_json_value());

                    json_content
                }
                Tagged::Adjacent {
                    tag_field,
                    content_field,
                } => {
                    let mut obj = JsonMap::new();
                    obj.insert(tag_field.to_string(), tag.as_json_value());
                    obj.insert(content_field.to_string(), json_content);
                    JsonValue::Object(obj)
                }
            }
        } else {
            // Untagged enum
            json_content
        };

        if let Some(repr) = &self.repr {
            return repr.into_repr(registry, json_value);
        }

        Ok(json_value)
    }
}
