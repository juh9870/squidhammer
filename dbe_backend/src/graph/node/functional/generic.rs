use crate::etype::conversion::EItemInfoAdapter;
use crate::etype::default::DefaultEValue;
use crate::etype::EDataType;
use crate::graph::node::generic::{GenericNodeField, GenericNodeFieldMut};
use crate::registry::optional_helpers::{none_of_type, unwrap_optional_value, wrap_in_option};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use arrayvec::ArrayVec;
use itertools::Itertools;
use miette::bail;

pub(super) const MAX_FIELDS: usize = 5;

pub(super) fn sync_generic_state<'a>(
    types: impl IntoIterator<Item = &'a mut Option<EDataType>>,
    type_indices: impl IntoIterator<Item = Option<usize>>,
) {
    let mut groups: [ArrayVec<usize, { MAX_FIELDS * 2 }>; MAX_FIELDS] = Default::default();
    let mut has_generics = false;
    for (idx, group) in type_indices
        .into_iter()
        .enumerate()
        .filter_map(|(idx, value)| value.map(|value| (idx, value)))
    {
        groups[group].push(idx);
        has_generics = true;
    }

    if !has_generics {
        return;
    }

    let mut types = types.into_iter().collect_vec();

    for indices in groups {
        let Some(ty) = indices.iter().filter_map(|i| *types[*i]).next() else {
            continue;
        };

        for idx in indices {
            *types[idx] = Some(ty);
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub(super) struct GenericValue<const N: usize>(pub EValue);

pub(super) trait GenericFieldAdapter {
    fn type_index() -> Option<usize>;
    fn field<'a>(registry: &ETypesRegistry, ty: &'a Option<EDataType>) -> GenericNodeField<'a>;
    fn field_mut<'a>(
        registry: &ETypesRegistry,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a>;
    fn try_from_evalue(
        registry: &ETypesRegistry,
        generic_ty: Option<EDataType>,
        value: &EValue,
    ) -> miette::Result<Self>
    where
        Self: Sized;
    fn into_evalue(
        self,
        registry: &ETypesRegistry,
        generic_ty: Option<EDataType>,
    ) -> miette::Result<EValue>;

    fn custom_default_value(registry: &ETypesRegistry) -> miette::Result<Option<DefaultEValue>> {
        let _ = (registry,);
        Ok(None)
    }
}

impl<const N: usize> GenericFieldAdapter for GenericValue<N> {
    fn type_index() -> Option<usize> {
        Some(N)
    }

    fn field<'a>(_registry: &ETypesRegistry, ty: &'a Option<EDataType>) -> GenericNodeField<'a> {
        GenericNodeField::Value(ty)
    }

    fn field_mut<'a>(
        _registry: &ETypesRegistry,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a> {
        GenericNodeFieldMut::Value(ty)
    }

    fn try_from_evalue(
        _registry: &ETypesRegistry,
        _generic_ty: Option<EDataType>,
        value: &EValue,
    ) -> miette::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(value.clone()))
    }

    fn into_evalue(
        self,
        _registry: &ETypesRegistry,
        _generic_ty: Option<EDataType>,
    ) -> miette::Result<EValue> {
        Ok(self.0)
    }
}

impl<const N: usize> GenericFieldAdapter for Vec<GenericValue<N>> {
    fn type_index() -> Option<usize> {
        Some(N)
    }

    fn field<'a>(_registry: &ETypesRegistry, ty: &'a Option<EDataType>) -> GenericNodeField<'a> {
        GenericNodeField::List(ty)
    }

    fn field_mut<'a>(
        _registry: &ETypesRegistry,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a> {
        GenericNodeFieldMut::List(ty)
    }

    fn try_from_evalue(
        _registry: &ETypesRegistry,
        _generic_ty: Option<EDataType>,
        value: &EValue,
    ) -> miette::Result<Self>
    where
        Self: Sized,
    {
        let EValue::List { values, id: _ } = value else {
            bail!("Expected list, got {:?}", value);
        };

        Ok(values
            .iter()
            .map(|value| GenericValue(value.clone()))
            .collect())
    }

    fn into_evalue(
        self,
        registry: &ETypesRegistry,
        generic_ty: Option<EDataType>,
    ) -> miette::Result<EValue> {
        let Some(ty) = generic_ty else {
            return Ok(EValue::List {
                values: vec![],
                id: registry.list_id_of(EDataType::null()),
            });
        };

        let values = self.into_iter().map(|value| value.0).collect_vec();

        Ok(EValue::List {
            values,
            id: registry.list_id_of(ty),
        })
    }
}

impl<const N: usize> GenericFieldAdapter for Option<GenericValue<N>> {
    fn type_index() -> Option<usize> {
        Some(N)
    }

    fn field<'a>(_registry: &ETypesRegistry, ty: &'a Option<EDataType>) -> GenericNodeField<'a> {
        GenericNodeField::Option(ty)
    }

    fn field_mut<'a>(
        _registry: &ETypesRegistry,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a> {
        GenericNodeFieldMut::Option(ty)
    }

    fn try_from_evalue(
        registry: &ETypesRegistry,
        _generic_ty: Option<EDataType>,
        value: &EValue,
    ) -> miette::Result<Self>
    where
        Self: Sized,
    {
        match unwrap_optional_value(registry, value)? {
            None => Ok(None),
            Some(value) => Ok(Some(GenericValue(value.clone()))),
        }
    }

    fn into_evalue(
        self,
        registry: &ETypesRegistry,
        generic_ty: Option<EDataType>,
    ) -> miette::Result<EValue> {
        let Some(ty) = generic_ty else {
            return Ok(none_of_type(registry, EDataType::null()));
        };

        Ok(wrap_in_option(registry, ty, self.map(|value| value.0)))
    }
}

impl<T: EItemInfoAdapter> GenericFieldAdapter for T {
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
        T::try_from_evalue(registry, value)
    }

    fn into_evalue(
        self,
        registry: &ETypesRegistry,
        _generic_ty: Option<EDataType>,
    ) -> miette::Result<EValue> {
        self.into_evalue(registry)
    }
}
