use crate::json_utils::repr::colors::ColorStringRepr;
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use miette::miette;
use parking_lot::RwLock;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use ustr::{Ustr, UstrMap};

mod colors;

#[allow(clippy::wrong_self_convention)]
pub trait JsonRepr: Send + Sync + Debug {
    /// Converts from the serialized data representation to the consumable data
    fn from_repr(
        &self,
        registry: &ETypesRegistry,
        data: &mut JsonValue,
        ignore_extra_fields: bool,
    ) -> miette::Result<JsonValue>;
    /// Converts from the consumable data to the serialized data representation
    fn into_repr(&self, registry: &ETypesRegistry, data: JsonValue) -> miette::Result<JsonValue>;
}

#[derive(Debug, Clone)]
pub struct Repr(Arc<dyn JsonRepr>);

impl JsonRepr for Repr {
    fn from_repr(
        &self,
        registry: &ETypesRegistry,
        data: &mut JsonValue,
        ignore_extra_fields: bool,
    ) -> miette::Result<JsonValue> {
        self.0.from_repr(registry, data, ignore_extra_fields)
    }

    fn into_repr(&self, registry: &ETypesRegistry, data: JsonValue) -> miette::Result<JsonValue> {
        self.0.into_repr(registry, data)
    }
}

impl FromStr for Repr {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        get_repr(&s.into()).ok_or_else(|| miette!("unknown repr `{}`", s))
    }
}

static REPR_REGISTRY: LazyLock<RwLock<UstrMap<Repr>>> = LazyLock::new(|| {
    RwLock::new({
        let mut map = UstrMap::default();

        map.insert("argb".into(), Repr(Arc::new(ColorStringRepr::ARGB)));
        map.insert("rgba".into(), Repr(Arc::new(ColorStringRepr::RGBA)));

        map
    })
});

pub fn get_repr(name: &Ustr) -> Option<Repr> {
    REPR_REGISTRY.read().get(name).cloned()
}
