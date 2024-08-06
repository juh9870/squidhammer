use crate::json_utils::repr::JsonRepr;
use crate::json_utils::{json_expected, JsonValue};
use crate::registry::ETypesRegistry;
use miette::miette;

#[derive(Debug)]
pub struct NumericRepr {
    is_ref: bool,
}

impl NumericRepr {
    pub const REF: NumericRepr = NumericRepr { is_ref: true };
    pub const ID: NumericRepr = NumericRepr { is_ref: false };
}

impl JsonRepr for NumericRepr {
    fn from_repr(
        &self,
        _registry: &ETypesRegistry,
        data: &mut JsonValue,
        _ignore_extra_fields: bool,
    ) -> miette::Result<JsonValue> {
        let str = json_expected(data.as_f64(), data, "number")?;

        let mut fields = serde_json::value::Map::new();

        // TODO: Duplicate ID check

        fields.insert("id".to_string(), str.into());

        Ok(fields.into())
    }

    fn into_repr(&self, _registry: &ETypesRegistry, data: JsonValue) -> miette::Result<JsonValue> {
        let obj = json_expected(data.as_object(), &data, "object")?;

        let id = obj
            .get("id")
            .ok_or_else(|| miette!("missing id field"))?
            .as_str()
            .ok_or_else(|| miette!("id field must be a string"))?;

        Ok(id.into())
    }
}
