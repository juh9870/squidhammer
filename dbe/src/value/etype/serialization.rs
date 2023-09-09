use crate::value::etype::registry::{EStructData, EStructField, EStructId, EStructRegistry};
use crate::value::etype::EDataType;
use anyhow::{anyhow, bail, Context};
use camino::Utf8Path;
use serde_json::Value;
use ustr::Ustr;

fn parse_string_thing(value: &Value) -> anyhow::Result<EDataType> {
    let value = value
        .as_str()
        .ok_or_else(|| anyhow!("Type definition must be a string"))?;

    anyhow::ensure!(!value.is_empty(), "Empty type name");

    Ok(match value {
        "boolean" => EDataType::Boolean,
        "number" => EDataType::Scalar,
        "string" => EDataType::String,
        "vec2" => EDataType::Vec2,
        ty => {
            let ty = EStructId::parse(ty)?;
            EDataType::Struct { ident: ty }
        }
    })
}

pub fn deserialize_thing(
    registry: &mut EStructRegistry,
    path: &Utf8Path,
    data: &Value,
) -> Result<EDataType, anyhow::Error> {
    let id = EStructId::from_path(path, registry.root_path())
        .context("While generating type identifier")?;
    match data {
        Value::Array(_) => {
            todo!("Thing arrays")
        }
        Value::Object(fields) => {
            let mut data = EStructData::new(id);
            for (name, value) in fields {
                let name_ident = Ustr::from(name);
                let ty = parse_string_thing(value)
                    .with_context(|| format!("While parsing field \"{name}\""))?;
                data.fields.push(EStructField {
                    name: name_ident,
                    ty,
                });
            }

            Ok(registry.register_struct(id, data))
        }
        _ => bail!("Thing root can only be an array or object"),
    }
}
