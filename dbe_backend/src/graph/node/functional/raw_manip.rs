use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::functional::generic::GenericValue;
use crate::graph::node::functional::values::AnyEValue;
use crate::graph::node::functional::{functional_node, C};
use crate::graph::node::ports::{port_types_compatible, NodePortType};
use crate::graph::node::NodeFactory;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use miette::{bail, miette};
use std::sync::Arc;
use strum::EnumIs;
use ustr::ustr;

#[derive(Debug, Copy, Clone, EnumIs)]
pub enum SwapValueResult {
    Swapped,
    InvalidType,
    FieldNotFound,
}

/// Swaps value of a field in a struct or an enum.
fn swap_value(
    registry: &ETypesRegistry,
    obj: &mut EValue,
    field: &str,
    value: &mut EValue,
) -> miette::Result<SwapValueResult> {
    match obj {
        EValue::Struct { fields, ident } => {
            if let Some(field_value) = fields.get_mut(&ustr(field)) {
                let in_info = EItemInfo::simple_type(value.ty());
                let out_info = EItemInfo::simple_type(field_value.ty());

                if !port_types_compatible(registry, &in_info, &out_info) {
                    return Ok(SwapValueResult::InvalidType);
                }

                let in_port: NodePortType = in_info.into();
                let out_port: NodePortType = out_info.into();

                *value = NodePortType::convert_value(
                    registry,
                    &in_port,
                    &out_port,
                    std::mem::replace(value, EValue::Null),
                )?;

                std::mem::swap(field_value, value);
                return Ok(SwapValueResult::Swapped);
            }

            let data = registry
                .get_struct(ident)
                .ok_or_else(|| miette!("struct not found"))?;

            for inline_field in &data.fields {
                if !inline_field.is_inline() {
                    continue;
                }

                let inline_value = fields
                    .get_mut(&inline_field.name)
                    .ok_or_else(|| miette!("!!INTERNAL ERROR!! field should be present"))?;

                let result = swap_value(registry, inline_value, field, value)?;
                if !result.is_field_not_found() {
                    return Ok(result);
                }
            }

            Ok(SwapValueResult::FieldNotFound)
        }
        EValue::Enum { data, .. } => swap_value(registry, data, field, value),
        _ => bail!("value is not a struct or an enum"),
    }
}

fn get_value(
    registry: &ETypesRegistry,
    value: &EValue,
    field: &str,
) -> miette::Result<Option<EValue>> {
    match value {
        EValue::Struct { fields, ident } => {
            if let Some(value) = fields.get(&ustr(field)) {
                return Ok(Some(value.clone()));
            }

            let data = registry
                .get_struct(ident)
                .ok_or_else(|| miette!("struct not found"))?;
            for inline_field in &data.fields {
                if !inline_field.is_inline() {
                    continue;
                }

                let inline_value = fields
                    .get(&inline_field.name)
                    .ok_or_else(|| miette!("!!INTERNAL ERROR!! field should be present"))?;

                if let Some(value) = get_value(registry, inline_value, field)? {
                    return Ok(Some(value));
                }
            }

            Ok(None)
        }
        EValue::Enum { data, .. } => get_value(registry, data, field),
        _ => bail!("value is not a struct or an enum"),
    }
}

pub(super) fn nodes() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        functional_node(
            |ctx: C, value: AnyEValue, field: String| {
                let value = value.0;
                let value = get_value(ctx.context.registry, &value, &field)?;
                Ok(value.map(AnyEValue))
            },
            "try_get_field",
            &["object", "field"],
            &["result"],
            &["utility.raw"],
        ),
        functional_node(
            |ctx: C, value: AnyEValue, field: String| {
                let value = value.0;
                let Some(value) = get_value(ctx.context.registry, &value, &field)? else {
                    bail!(
                        "field `{}` not found in object of type `{}`",
                        field,
                        value.ty().name()
                    )
                };

                Ok(AnyEValue(value))
            },
            "get_field",
            &["object", "field"],
            &["result"],
            &["utility.raw"],
        ),
        functional_node(
            |ctx: C, mut obj: GenericValue<0>, field: String, mut value: AnyEValue| {
                let success =
                    match swap_value(ctx.context.registry, &mut obj.0, &field, &mut value.0)? {
                        SwapValueResult::Swapped => true,
                        SwapValueResult::InvalidType | SwapValueResult::FieldNotFound => false,
                    };
                Ok((obj, success, success.then_some(value)))
            },
            "try_set_field",
            &["object", "field", "value"],
            &["object", "success", "old_value"],
            &["utility.raw"],
        ),
        functional_node(
            |ctx: C, mut obj: GenericValue<0>, field: String, mut value: AnyEValue| {
                match swap_value(ctx.context.registry, &mut obj.0, &field, &mut value.0)? {
                    SwapValueResult::Swapped => {}
                    SwapValueResult::InvalidType => {
                        bail!(
                            "type mismatch when setting field `{}` in object of type `{}`",
                            field,
                            obj.0.ty().name()
                        );
                    }
                    SwapValueResult::FieldNotFound => {
                        bail!(
                            "field `{}` not found in object of type `{}`",
                            field,
                            obj.0.ty().name()
                        );
                    }
                }
                Ok((obj, value))
            },
            "set_field",
            &["object", "field", "value"],
            &["object", "old_value"],
            &["utility.raw"],
        ),
        functional_node(
            |ctx: C, value: AnyEValue| -> miette::Result<Option<GenericValue<0>>> {
                let target_ty = ctx.output_types[0].unwrap_or_else(EDataType::null);
                let in_info = EItemInfo::simple_type(value.0.ty());
                let out_info = EItemInfo::simple_type(target_ty);

                if !port_types_compatible(ctx.context.registry, &in_info, &out_info) {
                    return Ok(None);
                }

                let in_port: NodePortType = in_info.into();
                let out_port: NodePortType = out_info.into();

                let converted = NodePortType::convert_value(
                    ctx.context.registry,
                    &in_port,
                    &out_port,
                    value.0,
                )?;
                Ok(Some(GenericValue(converted)))
            },
            "try_as_type",
            &["value"],
            &["result"],
            &["utility.raw"],
        ),
        functional_node(
            |ctx: C, value: AnyEValue| -> miette::Result<GenericValue<0>> {
                let target_ty = ctx.output_types[0].unwrap_or_else(EDataType::null);
                let in_port: NodePortType = EItemInfo::simple_type(value.0.ty()).into();
                let out_port: NodePortType = EItemInfo::simple_type(target_ty).into();

                let converted = NodePortType::convert_value(
                    ctx.context.registry,
                    &in_port,
                    &out_port,
                    value.0,
                )?;
                Ok(GenericValue(converted))
            },
            "as_type",
            &["value"],
            &["result"],
            &["utility.raw"],
        ),
        functional_node(
            |ctx: C, value: AnyEValue| {
                let EValue::Enum { data: _, variant } = value.0 else {
                    bail!("value is not an enum");
                };
                let Some(variant) = variant.variant(ctx.context.registry) else {
                    bail!("enum variant not found");
                };

                let tag = variant.get_tag_value();

                Ok(AnyEValue(tag.default_value()))
            },
            "enum_variant_tag",
            &["enum"],
            &["tag"],
            &["utility.raw"],
        ),
        functional_node(
            |_: C, value: AnyEValue| {
                let EValue::Enum { data, variant: _ } = value.0 else {
                    bail!("value is not an enum");
                };

                Ok(AnyEValue(*data))
            },
            "enum_inner_value",
            &["enum"],
            &["value"],
            &["utility.raw"],
        ),
        functional_node(
            |_: C, value: AnyEValue| {
                let EValue::Enum { data: _, variant } = value.0 else {
                    bail!("value is not an enum");
                };

                Ok(variant.variant_name().to_string())
            },
            "enum_variant_name",
            &["enum"],
            &["variant_name"],
            &["utility.raw"],
        ),
        functional_node(
            |_: C, a: AnyEValue, b: AnyEValue| a == b,
            "any_equals",
            &["a", "b"],
            &["a == b"],
            &["utility.raw"],
        ),
        functional_node(
            |_: C, a: AnyEValue, b: AnyEValue| a != b,
            "any_not_equals",
            &["a", "b"],
            &["a != b"],
            &["utility.raw"],
        ),
    ]
}
