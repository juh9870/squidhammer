use crate::etype::EDataType;
use crate::registry::ETypesRegistry;
use crate::value::EValue;

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
