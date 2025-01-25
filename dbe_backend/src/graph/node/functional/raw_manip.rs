use crate::registry::ETypesRegistry;
use crate::value::EValue;
use miette::{bail, miette};
use ustr::ustr;

/// Swaps value of a field in a struct or an enum.
pub(super) fn swap_value(
    registry: &ETypesRegistry,
    obj: &mut EValue,
    field: &str,
    value: &mut EValue,
) -> miette::Result<bool> {
    match obj {
        EValue::Struct { fields, ident } => {
            if let Some(field_value) = fields.get_mut(&ustr(field)) {
                std::mem::swap(field_value, value);
                return Ok(true);
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

                if swap_value(registry, inline_value, field, value)? {
                    return Ok(true);
                }
            }

            Ok(false)
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
