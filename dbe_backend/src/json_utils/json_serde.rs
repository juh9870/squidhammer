use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;

pub trait JsonSerde {
    type State<'a>;
    /// Writes node state to json
    fn write_json(
        &self,
        registry: &ETypesRegistry,
        external_state: Self::State<'_>,
    ) -> miette::Result<JsonValue> {
        let _ = (registry, external_state);
        Ok(JsonValue::Null)
    }
    /// Loads node state from json
    fn parse_json(
        &mut self,
        registry: &ETypesRegistry,
        external_state: Self::State<'_>,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        let _ = (registry, external_state, value);
        Ok(())
    }
}
