use miette::miette;
pub use serde_json::Map as JsonMap;
use serde_json::Value;
pub use serde_json::Value as JsonValue;

pub mod formatter;
pub mod json_serde;
pub mod repr;

pub fn json_kind(value: &JsonValue) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub fn json_expected<T>(value: Option<T>, json: &JsonValue, ty: &str) -> miette::Result<T> {
    value.ok_or_else(|| {
        miette!(
            "invalid data type. Expected {} but got {}",
            ty,
            json_kind(json)
        )
    })
}
