use crate::etype::eobject::EObject;
use crate::etype::EDataType;
use crate::registry::{ETypesRegistry, OPTIONAL_ID};
use crate::value::EValue;
use miette::bail;

pub fn wrap_in_some(registry: &ETypesRegistry, value: EValue) -> EValue {
    let data = registry.option_data_of(value.ty());
    let some_variant = data.variant_ids()[1];
    assert_eq!(some_variant.variant_name(), "Some");
    EValue::Enum {
        variant: some_variant,
        data: Box::new(value),
    }
}

pub fn none_of_type(registry: &ETypesRegistry, ty: EDataType) -> EValue {
    let data = registry.option_data_of(ty);
    let none_variant = data.variant_ids()[0];
    assert_eq!(none_variant.variant_name(), "None");
    EValue::Enum {
        variant: none_variant,
        data: Box::new(EValue::Null),
    }
}

pub fn unwrap_optional_value<'a>(
    registry: &ETypesRegistry,
    value: &'a EValue,
) -> miette::Result<Option<&'a EValue>> {
    let EValue::Enum {
        variant,
        data: value,
    } = value
    else {
        bail!("Expected an optional value, got {}", value.ty().name());
    };

    let Some(data) = registry.get_enum(&variant.enum_id()) else {
        bail!("Expected an optional value, got {}", value.ty().name());
    };

    if data.generic_parent_id() != Some(*OPTIONAL_ID) {
        bail!("Expected an optional value, got {}", value.ty().name());
    }

    match variant.variant_name().as_str() {
        "Some" => Ok(Some(value)),
        "None" => Ok(None),
        _ => panic!("Unexpected variant name"),
    }
}
