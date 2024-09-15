use crate::etype::eenum::pattern::EnumPattern;
use crate::json_utils::repr::colors::ColorStringRepr;
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use crate::validation::Validator;
use miette::miette;
use parking_lot::RwLock;
use std::borrow::Cow;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use ustr::{Ustr, UstrMap};

mod colors;
mod ids;

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

    /// Custom enum pattern for this repr. Leave none if this repr does not change the shape of the data
    fn enum_pat(&self) -> Option<EnumPattern>;

    /// Custom validators imposed by this repr
    fn validators(&self) -> Cow<'static, [Validator]> {
        Cow::Borrowed(&[])
    }
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

    fn enum_pat(&self) -> Option<EnumPattern> {
        self.0.enum_pat()
    }

    fn validators(&self) -> Cow<'static, [Validator]> {
        self.0.validators()
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
        map.insert("ids/numeric".into(), Repr(Arc::new(ids::numeric::Id)));
        map.insert("ids/numeric_ref".into(), Repr(Arc::new(ids::numeric::Ref)));

        map
    })
});

pub fn get_repr(name: &Ustr) -> Option<Repr> {
    REPR_REGISTRY.read().get(name).cloned()
}

macro_rules! transparent {
    ($field_name:literal, $cast_fn:path, $expected:literal, $pattern:expr) => {
        fn from_repr(
            &self,
            _registry: &$crate::registry::ETypesRegistry,
            data: &mut crate::json_utils::JsonValue,
            _ignore_extra_fields: bool,
        ) -> miette::Result<crate::json_utils::JsonValue> {
            let str = $crate::json_utils::json_expected($cast_fn(data), data, $expected)?;

            let mut fields = serde_json::value::Map::new();

            fields.insert($field_name.to_string(), str.into());

            Ok(fields.into())
        }

        fn into_repr(
            &self,
            _registry: &$crate::registry::ETypesRegistry,
            data: JsonValue,
        ) -> miette::Result<crate::json_utils::JsonValue> {
            let obj = $crate::json_utils::json_expected(data.as_object(), &data, "object")?;

            let id = $cast_fn(
                obj.get(stringify!($field_name))
                    .ok_or_else(|| miette::miette!("missing `{}` field", $field_name))?,
            )
            .ok_or_else(|| miette::miette!("`{}` field must be a {}", $field_name, $expected))?;

            Ok(id.into())
        }

        fn enum_pat(&self) -> Option<EnumPattern> {
            Some($pattern)
        }
    };
}

pub(crate) use transparent;
