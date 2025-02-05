use crate::etype::conversion::{EItemInfoAdapter, ManualEItemInfoAdapter};
use crate::etype::default::DefaultEValue;
use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::functional::generic::GenericFieldAdapter;
use crate::graph::node::generic::{GenericNodeField, GenericNodeFieldMut};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use std::marker::PhantomData;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct AnyEValue(pub EValue);

impl TryFrom<&EValue> for AnyEValue {
    type Error = miette::Report;

    fn try_from(value: &EValue) -> Result<Self, Self::Error> {
        Ok(Self(value.clone()))
    }
}

impl From<AnyEValue> for EValue {
    fn from(value: AnyEValue) -> Self {
        value.0
    }
}

impl EItemInfoAdapter for AnyEValue {
    fn edata_type(_registry: &ETypesRegistry) -> EItemInfo {
        EItemInfo::simple_type(EDataType::Unknown)
    }
}

pub struct CustomEValue<T: ManualEItemInfoAdapter>(pub EValue, pub PhantomData<fn() -> T>);

impl<T: ManualEItemInfoAdapter> TryFrom<&EValue> for CustomEValue<T> {
    type Error = miette::Report;

    fn try_from(value: &EValue) -> Result<Self, Self::Error> {
        Ok(Self(value.clone(), Default::default()))
    }
}

impl<T: ManualEItemInfoAdapter> From<CustomEValue<T>> for EValue {
    fn from(value: CustomEValue<T>) -> Self {
        value.0
    }
}

impl<T: ManualEItemInfoAdapter> EItemInfoAdapter for CustomEValue<T> {
    fn edata_type(_registry: &ETypesRegistry) -> EItemInfo {
        T::edata_type(_registry)
    }
}

pub trait DefaultValueProvider {
    fn default_value(registry: &ETypesRegistry) -> miette::Result<DefaultEValue>;
}

pub struct WithDefault<T: EItemInfoAdapter, Default: DefaultValueProvider>(
    pub T,
    pub PhantomData<fn() -> Default>,
);

impl<T: EItemInfoAdapter, D: DefaultValueProvider> GenericFieldAdapter for WithDefault<T, D> {
    fn type_index() -> Option<usize> {
        None
    }

    fn field<'a>(registry: &ETypesRegistry, _ty: &'a Option<EDataType>) -> GenericNodeField<'a> {
        GenericNodeField::Fixed(T::edata_type(registry).ty())
    }

    fn field_mut<'a>(
        registry: &ETypesRegistry,
        _ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a> {
        GenericNodeFieldMut::Fixed(T::edata_type(registry).ty())
    }

    fn try_from_evalue(
        registry: &ETypesRegistry,
        _generic_ty: Option<EDataType>,
        value: &EValue,
    ) -> miette::Result<Self>
    where
        Self: Sized,
    {
        T::try_from_evalue(registry, value).map(|v| Self(v, Default::default()))
    }

    fn into_evalue(
        self,
        registry: &ETypesRegistry,
        _generic_ty: Option<EDataType>,
    ) -> miette::Result<EValue> {
        self.0.into_evalue(registry)
    }

    fn custom_default_value(registry: &ETypesRegistry) -> miette::Result<Option<DefaultEValue>> {
        D::default_value(registry).map(Some)
    }
}

pub mod default {
    use crate::etype::default::DefaultEValue;
    use crate::graph::node::functional::values::DefaultValueProvider;
    use crate::registry::ETypesRegistry;
    use crate::value::EValue;

    pub struct DefaultENumber<const N: i64>;

    impl<const N: i64> DefaultValueProvider for DefaultENumber<N> {
        fn default_value(_registry: &ETypesRegistry) -> miette::Result<DefaultEValue> {
            Ok(EValue::from(N as f64).into())
        }
    }

    pub struct DefaultTrueBool;

    impl DefaultValueProvider for DefaultTrueBool {
        fn default_value(_registry: &ETypesRegistry) -> miette::Result<DefaultEValue> {
            Ok(EValue::from(true).into())
        }
    }
}
