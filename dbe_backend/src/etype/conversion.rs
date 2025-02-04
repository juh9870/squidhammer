use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::registry::optional_helpers::{unwrap_optional_value, wrap_in_option};
use crate::registry::ETypesRegistry;
use crate::value::{ENumber, EValue};
use miette::bail;

pub trait ValueAdapter {
    fn try_from_evalue(registry: &ETypesRegistry, value: &EValue) -> miette::Result<Self>
    where
        Self: Sized;
    fn into_evalue(self, registry: &ETypesRegistry) -> miette::Result<EValue>;
}

impl<T: Into<EValue> + for<'a> TryFrom<&'a EValue, Error = miette::Report>> ValueAdapter for T {
    fn try_from_evalue(_registry: &ETypesRegistry, value: &EValue) -> miette::Result<Self> {
        Self::try_from(value)
    }

    fn into_evalue(self, _registry: &ETypesRegistry) -> miette::Result<EValue> {
        Ok(self.into())
    }
}

/// Like [`EItemInfoAdapter`], but for nodes that have custom handling of their data type.
pub trait ManualEItemInfoAdapter {
    fn edata_type(registry: &ETypesRegistry) -> EItemInfo;
}

impl ManualEItemInfoAdapter for () {
    fn edata_type(_registry: &ETypesRegistry) -> EItemInfo {
        EItemInfo::simple_type(EDataType::Unknown)
    }
}

pub trait EItemInfoAdapter: ValueAdapter {
    /// Get the data type of this value.
    fn edata_type(registry: &ETypesRegistry) -> EItemInfo;
}

impl EItemInfoAdapter for ENumber {
    fn edata_type(_registry: &ETypesRegistry) -> EItemInfo {
        EItemInfo::simple_type(EDataType::Number)
    }
}

impl EItemInfoAdapter for bool {
    fn edata_type(_registry: &ETypesRegistry) -> EItemInfo {
        EItemInfo::simple_type(EDataType::Boolean)
    }
}

impl EItemInfoAdapter for String {
    fn edata_type(_registry: &ETypesRegistry) -> EItemInfo {
        EItemInfo::simple_type(EDataType::String)
    }
}

impl<T: EItemInfoAdapter> EItemInfoAdapter for Vec<T> {
    fn edata_type(registry: &ETypesRegistry) -> EItemInfo {
        EItemInfo::simple_type(registry.list_of(T::edata_type(registry).ty()))
    }
}

impl<T: EItemInfoAdapter> ValueAdapter for Vec<T> {
    fn try_from_evalue(registry: &ETypesRegistry, value: &EValue) -> miette::Result<Self>
    where
        Self: Sized,
    {
        let EValue::List { values, id: _ } = value else {
            bail!("Expected list, got {:?}", value);
        };

        values
            .iter()
            .map(|value| T::try_from_evalue(registry, value))
            .collect()
    }

    fn into_evalue(self, registry: &ETypesRegistry) -> miette::Result<EValue> {
        let values = self
            .into_iter()
            .map(|value| value.into_evalue(registry))
            .collect::<miette::Result<Vec<_>>>()?;

        let info = Self::edata_type(registry);

        Ok(EValue::List {
            values,
            id: registry.list_id_of(info.ty()),
        })
    }
}

impl<T: EItemInfoAdapter> EItemInfoAdapter for Option<T> {
    fn edata_type(registry: &ETypesRegistry) -> EItemInfo {
        EItemInfo::simple_type(EDataType::Object {
            ident: registry.option_id_of(T::edata_type(registry).ty()),
        })
    }
}

impl<T: EItemInfoAdapter> ValueAdapter for Option<T> {
    fn try_from_evalue(registry: &ETypesRegistry, value: &EValue) -> miette::Result<Self>
    where
        Self: Sized,
    {
        match unwrap_optional_value(registry, value)? {
            None => Ok(None),
            Some(value) => T::try_from_evalue(registry, value).map(Some),
        }
    }

    fn into_evalue(self, registry: &ETypesRegistry) -> miette::Result<EValue> {
        Ok(wrap_in_option(
            registry,
            T::edata_type(registry).ty(),
            self.map(|value| value.into_evalue(registry)).transpose()?,
        ))
    }
}
