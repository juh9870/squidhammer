use crate::value::etype::registry::eenum::EEnumData;
use crate::value::etype::registry::estruct::{EStructData, EStructField};
use crate::value::etype::registry::{EObjectType, ETypesRegistry, ETypetId};
use crate::value::etype::EDataType;
use anyhow::{anyhow, bail, Context};
use camino::Utf8Path;
use serde_json::Value;
use types_parsing::parse_type_string;
use ustr::Ustr;

mod types_parsing;

pub fn parse_type(value: &Value) -> anyhow::Result<EDataType> {
    let value = value
        .as_str()
        .ok_or_else(|| anyhow!("Type definition must be a string"))?;

    anyhow::ensure!(!value.is_empty(), "Empty type name");

    parse_type_string(value)
}

pub fn deserialize_thing(
    registry: &mut ETypesRegistry,
    id: ETypetId,
    data: &Value,
) -> Result<EObjectType, anyhow::Error> {
    match data {
        Value::Array(items) => Ok(EObjectType::Enum(
            EEnumData::deserialize(registry, id, items)
                .with_context(|| format!("Parsing enum `{id}`"))?,
        )),
        Value::Object(fields) => Ok(EObjectType::Struct(
            EStructData::deserialize(registry, id, fields)
                .with_context(|| format!("Parsing struct `{id}`"))?,
        )),
        _ => bail!("Thing root can only be an array or object"),
    }
}
