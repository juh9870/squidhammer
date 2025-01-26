use crate::etype::eitem::EItemInfo;
use crate::graph::node::ports::{port_types_compatible, NodePortType};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use miette::{bail, miette};
use strum::EnumIs;
use ustr::ustr;

#[derive(Debug, Copy, Clone, EnumIs)]
pub enum SwapValueResult {
    Swapped,
    InvalidType,
    FieldNotFound,
}

/// Swaps value of a field in a struct or an enum.
pub(super) fn swap_value(
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

pub(super) fn get_value(
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
