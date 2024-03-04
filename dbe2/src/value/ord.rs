use crate::value::{EValue, EValueDiscriminants};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

/// Ordering for the internal usages. May change between crate versions,
/// and should not be relied upon for any persistent store
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct EValueOrd(EValue);

impl PartialOrd for EValueOrd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EValueOrd {
    fn cmp(&self, other: &Self) -> Ordering {
        EValueOrdRef(&self.0).cmp(&EValueOrdRef(&other.0))
    }
}

impl Display for EValueOrd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct EValueOrdRef<'a>(&'a EValue);

impl<'a> PartialOrd for EValueOrdRef<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<'a> Ord for EValueOrdRef<'a> {
    //noinspection DuplicatedCode
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by Discriminants
        EValueDiscriminants::from(self.0)
            .cmp(&EValueDiscriminants::from(other.0))
            .then_with(|| match (self.0, other.0) {
                // Nulls are always equal
                (EValue::Null, EValue::Null) => Ordering::Equal,
                // Booleans, Numbers and Strings are compared by value
                (EValue::Boolean { value: a }, EValue::Boolean { value: b }) => a.cmp(b),
                (EValue::Number { value: a }, EValue::Number { value: b }) => a.cmp(b),
                (EValue::String { value: a }, EValue::String { value: b }) => a.cmp(b),
                // Structs are compared by ID first, and by fields second
                (
                    EValue::Struct { ident, fields },
                    EValue::Struct {
                        ident: other_ident,
                        fields: other_fields,
                    },
                ) => ident.ord().cmp(&other_ident.ord()).then_with(|| {
                    fields
                        .iter()
                        .map(|(k, v)| (k, EValueOrdRef(v)))
                        .cmp(other_fields.iter().map(|(k, v)| (k, EValueOrdRef(v))))
                }),
                // Id and Ref are compared by type first, and by value second
                (
                    EValue::Ref { ty, value } | EValue::Id { ty, value },
                    EValue::Ref {
                        ty: other_ty,
                        value: other_value,
                    }
                    | EValue::Id {
                        ty: other_ty,
                        value: other_value,
                    },
                ) => ty
                    .ord()
                    .cmp(&other_ty.ord())
                    .then_with(|| value.map(|e| e.ord()).cmp(&other_value.map(|e| e.ord()))),
                // Enums are compared by variant first, and by value second
                (
                    EValue::Enum { variant, data },
                    EValue::Enum {
                        variant: other_variant,
                        data: other_data,
                    },
                ) => variant
                    .ord()
                    .cmp(&other_variant.ord())
                    .then_with(|| EValueOrdRef(data).cmp(&EValueOrdRef(other_data))),
                // Lists are compared by ID first, and by values second
                (
                    EValue::List { id, values },
                    EValue::List {
                        id: other_id,
                        values: other_values,
                    },
                ) => id.ord().cmp(&other_id.ord()).then_with(|| {
                    values
                        .iter()
                        .map(EValueOrdRef)
                        .cmp(other_values.iter().map(EValueOrdRef))
                }),
                // Maps are compared by ID first, and by values second
                (
                    EValue::Map { id, values },
                    EValue::Map {
                        id: other_id,
                        values: other_values,
                    },
                ) => id.ord().cmp(&other_id.ord()).then_with(|| {
                    values
                        .iter()
                        .map(|(k, v)| (k, EValueOrdRef(v)))
                        .cmp(other_values.iter().map(|(k, v)| (k, EValueOrdRef(v))))
                }),
                (
                    EValue::Null
                    | EValue::Boolean { .. }
                    | EValue::Number { .. }
                    | EValue::String { .. }
                    | EValue::Struct { .. }
                    | EValue::Id { .. }
                    | EValue::Ref { .. }
                    | EValue::Enum { .. }
                    | EValue::List { .. }
                    | EValue::Map { .. },
                    _,
                ) => {
                    unreachable!("Already compared by ordinal before")
                }
            })
    }
}
