use crate::value::etype::registry::eenum::EEnumData;
use crate::value::etype::registry::eitem::{EItemType, EItemTypeTrait};
use crate::value::etype::registry::estruct::EStructData;
use crate::value::etype::registry::serialization::deserialize_thing;
use crate::value::etype::EDataType;
use crate::value::{EValue, JsonValue};
use anyhow::{anyhow, bail, Context};
use camino::{Utf8Path, Utf8PathBuf};
use egui_node_graph::DataTypeTrait;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use serde::{Deserializer, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use ustr::{Ustr, UstrMap};

pub mod eenum;
pub mod eitem;
pub mod estruct;
pub mod serialization;

#[derive(Debug, Clone)]
pub enum EObjectType {
    Struct(EStructData),
    Enum(EEnumData),
}

impl EObjectType {
    pub fn as_struct(&self) -> Option<&EStructData> {
        if let EObjectType::Struct(data) = self {
            return Some(data);
        }
        None
    }
    pub fn as_enum(&self) -> Option<&EEnumData> {
        if let EObjectType::Enum(data) = self {
            return Some(data);
        }
        None
    }
}

#[derive(Debug, Clone)]
enum RegistryItem {
    Raw(String),
    DeserializationInProgress,
    Ready(EObjectType),
}

impl RegistryItem {
    #[inline(always)]
    pub fn expect_ready(&self) -> &EObjectType {
        match self {
            RegistryItem::Ready(item) => item,
            _ => panic!("Registry item is not ready when expected"),
        }
    }
}

#[derive(Debug)]
pub struct ETypesRegistry {
    root: Utf8PathBuf,
    types: FxHashMap<ETypeId, RegistryItem>,
    last_id: u64,
}

impl ETypesRegistry {
    pub fn from_raws(
        root: Utf8PathBuf,
        data: impl IntoIterator<Item = (ETypeId, String)>,
    ) -> anyhow::Result<Self> {
        let iter = data.into_iter();

        let types: FxHashMap<ETypeId, RegistryItem> = iter
            .map(|(id, v)| {
                Result::<(ETypeId, RegistryItem), anyhow::Error>::Ok((id, RegistryItem::Raw(v)))
            })
            .try_collect()
            .context("While grouping entries")?;

        let reg = Self {
            root,
            types,
            last_id: 0,
        };

        reg.deserialize_all().context("While deserializing types")
    }

    pub fn debug_dump(&self) {
        dbg!(&self.types);
        dbg!(&self.root);
    }

    pub fn all_objects(&self) -> impl Iterator<Item = &EObjectType> {
        self.types.values().map(|e| e.expect_ready())
    }

    pub fn get_object(&self, id: &ETypeId) -> Option<&EObjectType> {
        self.types.get(id).map(RegistryItem::expect_ready)
    }

    pub fn get_struct(&self, id: &ETypeId) -> Option<&EStructData> {
        self.types
            .get(id)
            .and_then(|e| e.expect_ready().as_struct())
    }

    pub fn get_enum(&self, id: &ETypeId) -> Option<&EEnumData> {
        self.types.get(id).and_then(|e| e.expect_ready().as_enum())
    }

    pub fn register_struct(&mut self, id: ETypeId, data: EStructData) -> EDataType {
        self.types
            .insert(id, RegistryItem::Ready(EObjectType::Struct(data)));
        EDataType::Object { ident: id }
    }

    pub fn register_enum(&mut self, id: ETypeId, data: EEnumData) -> EDataType {
        self.types
            .insert(id, RegistryItem::Ready(EObjectType::Enum(data)));
        EDataType::Object { ident: id }
    }

    pub fn make_generic(
        &mut self,
        id: ETypeId,
        arguments: UstrMap<EItemType>,
    ) -> anyhow::Result<EDataType> {
        let long_id = {
            let args = arguments
                .iter()
                .map(|e| format!("{}={}", e.0, e.1.ty().name()))
                .sorted()
                .join(",");
            ETypeId::Persistent(format!("{id}<{args}>").into())
        };
        if self.types.contains_key(&long_id) {
            return Ok(EDataType::Object { ident: long_id });
        }

        let obj = self
            .get_object(&id)
            .with_context(|| format!("Failed to find object with id {}", id))?;

        let check_generics = |args: &Vec<Ustr>| {
            if args.len() != arguments.len() {
                bail!(
                    "Object {id} expects {} generic arguments, but {} were provided",
                    args.len(),
                    arguments.len()
                )
            }

            Ok(())
        };

        match obj {
            EObjectType::Struct(data) => {
                check_generics(&data.generic_arguments)?;
                let obj = data.apply_generics(&arguments)?;
                Ok(self.register_struct(long_id, obj))
            }
            EObjectType::Enum(data) => {
                check_generics(&data.generic_arguments)?;
                let obj = data.apply_generics(&arguments)?;
                Ok(self.register_enum(long_id, obj))
            }
        }
    }

    pub fn next_temp_id(&mut self) -> ETypeId {
        self.last_id += 1;
        ETypeId::Temp(self.last_id)
    }

    pub fn default_value(&self, ident: &ETypeId) -> EValue {
        let Some(data) = self.types.get(ident) else {
            return EValue::Unknown {
                value: JsonValue::Null,
            };
        };

        match data.expect_ready() {
            EObjectType::Struct(data) => data.default_value(self),
            EObjectType::Enum(data) => data.default_value(self),
        }
    }

    pub fn root_path(&self) -> &Utf8Path {
        self.root.as_path()
    }

    // fn register_raw_json_object(&mut self, id: ETypetId, data: JsonValue) -> EDataType {
    //     self.types.insert(*id, RegistryItem::Raw(data));
    //     EDataType::Object { ident: id }
    // }

    fn fetch_or_deserialize(&mut self, id: ETypeId) -> anyhow::Result<&EObjectType> {
        let data = self
            .types
            .get_mut(&id)
            .with_context(|| format!("Type `{id}` is not defined"))?;

        match data {
            RegistryItem::Ready(_) => {
                return Ok(self
                    .types
                    .get(&id)
                    .expect("Should be present")
                    .expect_ready());
            }
            RegistryItem::DeserializationInProgress => {
                bail!("Recursion error! Type `{id}` is in process of getting evaluated")
            }
            RegistryItem::Raw(_) => {} // handled next
        };

        let RegistryItem::Raw(old) =
            std::mem::replace(data, RegistryItem::DeserializationInProgress)
        else {
            panic!("Item should be raw")
        };
        let ready = RegistryItem::Ready(deserialize_thing(self, id, &old)?);
        self.types.insert(id, ready);
        Ok(self
            .types
            .get(&id)
            .expect("Item should be present")
            .expect_ready())
    }

    fn deserialize_all(mut self) -> anyhow::Result<Self> {
        let keys = self.types.keys().copied().collect_vec();
        for id in keys {
            self.fetch_or_deserialize(id)
                .with_context(|| format!("While deserializing `{id}`"))?;
        }

        debug_assert!(
            self.types
                .values()
                .all(|e| matches!(e, RegistryItem::Ready(_))),
            "All items should be deserialized"
        );

        Ok(self)
    }

    // MAYBE?: use https://github.com/compenguy/ngrammatic for hints
    fn assert_defined(&self, id: &ETypeId) -> anyhow::Result<()> {
        if !self.types.contains_key(id) {
            bail!("Type `{id}` is not defined")
        }
        Ok(())
    }
}

pub fn namespace_errors(namespace: &str) -> Option<(usize, char)> {
    namespace
        .chars()
        .find_position(|c| !matches!(c, 'a'..='z' | '0'..='9' | '_'))
}
pub fn path_errors(namespace: &str) -> Option<(usize, char)> {
    namespace
        .chars()
        .find_position(|c| !matches!(c, 'a'..='z' | '0'..='9' | '_' | '/'))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ETypeId {
    Persistent(Ustr),
    Temp(u64),
}

impl serde::Serialize for ETypeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ETypeId::Persistent(id) => id.serialize(serializer),
            ETypeId::Temp(id) => Err(serde::ser::Error::custom(format!(
                "temporary ETypetId can't be serialized: {}",
                id
            ))),
        }
    }
}

struct ETypeIdVisitor;

impl<'de> serde::de::Visitor<'de> for ETypeIdVisitor {
    type Value = ETypeId;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match ETypeId::parse(v) {
            Ok(data) => Ok(data),
            Err(err) => Err(serde::de::Error::custom(
                err.to_string().to_ascii_lowercase(),
            )),
        }
    }
}

impl<'de> serde::Deserialize<'de> for ETypeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(ETypeIdVisitor)
    }
}

impl ETypeId {
    pub fn parse(data: &str) -> anyhow::Result<Self> {
        let (namespace, path): (&str, &str) = data
            .split(':')
            .collect_tuple()
            .ok_or_else(|| anyhow!("Type path must be in a form of `namespace:path`"))?;

        if namespace.is_empty() {
            bail!("Namespace can't be empty")
        }

        if path.is_empty() {
            bail!("Path can't be empty")
        }

        if let Some((i, c)) = namespace_errors(namespace) {
            bail!("Invalid symbol `{c}` in namespace, at position {i}")
        }

        if let Some((i, c)) = path_errors(path) {
            bail!(
                "Invalid symbol `{c}` in path, at position {}",
                i + namespace.len() + 1
            )
        }

        Ok(ETypeId::Persistent(data.into()))
    }

    pub fn from_path(path: &Utf8Path, types_root: &Utf8Path) -> anyhow::Result<Self> {
        let sub_path = path
            .strip_prefix(types_root)
            .map_err(|_| anyhow!("Thing is outside of types root folder.\nThing: `{path}`"))?
            .components()
            .collect_vec();
        if sub_path.len() < 2 {
            bail!("Things can't be placed in a root of types folder")
        }

        let mut segments = sub_path.into_iter();
        let namespace = segments
            .next()
            .expect("Namespace should be present")
            .to_string();

        if let Some((i, c)) = namespace_errors(&namespace) {
            bail!("Namespace folder contains invalid character `{c}` at position {i}")
        }

        let segments: Vec<String> = segments
            .with_position()
            .map(|(pos, path)| {
                let str = if matches!(pos, itertools::Position::Last | itertools::Position::Only) {
                    let p: &Utf8Path = path.as_ref();
                    p.file_stem().ok_or_else(||anyhow!("Final path segment has an empty filename"))?.to_string()
                } else {
                    path.to_string()
                };
                if let Some((i, c)) = path_errors(&str) {
                    bail!("Path folder or file contains invalid symbol `{c}` at position {i} in segment `{path}`")
                }

                Ok(str)
            })
            .try_collect()?;

        let path = segments.join("/");

        if path.is_empty() {
            bail!("Things can't be placed in a root of types folder")
        }

        Self::parse(&format!("{namespace}:{path}"))
    }
    // #[inline(always)]
    // pub fn raw(&self) -> &Ustr {
    //     &self.0
    // }
}

impl FromStr for ETypeId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ETypeId::parse(s)
    }
}

impl Display for ETypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ETypeId::Persistent(id) => write!(f, "{}", id),
            ETypeId::Temp(id) => write!(f, "$temp:{}", id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ETypeId;
    use rstest::rstest;

    #[rstest]
    #[case("namespace:id")]
    #[case("namespace_123:a1/a2/a3/4/5/6")]
    #[case("eh:objects/faction")]
    fn should_parse_type_id(#[case] id: &str) {
        assert!(ETypeId::parse(id).is_ok())
    }

    #[test]
    fn should_fail_empty() {
        assert!(ETypeId::parse("").is_err())
    }

    #[test]
    fn should_fail_no_colon() {
        assert!(ETypeId::parse("some_name").is_err())
    }

    #[test]
    fn should_fail_empty_namespace() {
        assert!(ETypeId::parse(":some_name").is_err())
    }

    #[test]
    fn should_fail_empty_path() {
        assert!(ETypeId::parse("some_name:").is_err())
    }

    #[test]
    fn should_fail_slashes_in_namespace() {
        assert!(ETypeId::parse("namespace/other:path").is_err())
    }

    #[test]
    fn should_fail_capitalized() {
        assert!(ETypeId::parse("namespace/Path").is_err());
        assert!(ETypeId::parse("Namespace/path").is_err());
    }

    #[rstest]
    #[case("name space:id")]
    #[case("namespace:path.other")]
    #[case("namespace:path-other")]
    fn should_fail_invalid_characters(#[case] id: &str) {
        assert!(ETypeId::parse(id).is_err())
    }
}
