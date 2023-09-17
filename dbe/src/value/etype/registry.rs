use crate::value::etype::registry::eenum::EEnumData;
use crate::value::etype::registry::estruct::EStructData;
use crate::value::etype::registry::serialization::deserialize_thing;
use crate::value::etype::EDataType;
use crate::value::{EValue, JsonValue};
use anyhow::{anyhow, bail, Context};
use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use serde_json::Value;
use std::fmt::{Display, Formatter};
use ustr::{Ustr, UstrMap};

pub mod eenum;
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
    Raw(Value),
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
    types: UstrMap<RegistryItem>,
}

impl ETypesRegistry {
    pub fn from_raws(
        root: Utf8PathBuf,
        data: impl IntoIterator<Item = (Utf8PathBuf, JsonValue)>,
    ) -> anyhow::Result<Self> {
        let iter = data.into_iter();

        let types: UstrMap<RegistryItem> = iter
            .map(|(path, v)| {
                let id = ETypetId::from_path(&path, &root).with_context(|| {
                    format!("While generating type identifier for file `{path}`")
                })?;
                anyhow::Result::<(Ustr, RegistryItem)>::Ok((*id.raw(), RegistryItem::Raw(v)))
            })
            .try_collect()?;

        let reg = Self { root, types };

        reg.deserialize_all()
    }

    // pub fn types(&self) -> &UstrMap<EObjectType> {
    //     &self.types
    // }

    pub fn all_objects(&self) -> impl Iterator<Item = &EObjectType> {
        self.types.values().map(|e| e.expect_ready())
    }

    pub fn get_object(&self, id: &ETypetId) -> Option<&EObjectType> {
        self.types.get(id.raw()).map(RegistryItem::expect_ready)
    }

    pub fn get_struct(&self, id: &ETypetId) -> Option<&EStructData> {
        self.types
            .get(id.raw())
            .and_then(|e| e.expect_ready().as_struct())
    }

    pub fn get_enum(&self, id: &ETypetId) -> Option<&EEnumData> {
        self.types
            .get(id.raw())
            .and_then(|e| e.expect_ready().as_enum())
    }

    pub fn register_struct(&mut self, id: ETypetId, data: EStructData) -> EDataType {
        self.types
            .insert(*id.raw(), RegistryItem::Ready(EObjectType::Struct(data)));
        EDataType::Object { ident: id }
    }

    pub fn register_enum(&mut self, id: ETypetId, data: EEnumData) -> EDataType {
        self.types
            .insert(*id.raw(), RegistryItem::Ready(EObjectType::Enum(data)));
        EDataType::Object { ident: id }
    }

    pub fn default_value(&self, ident: &ETypetId) -> EValue {
        let Some(data) = self.types.get(ident.raw()) else {
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

    fn register_raw_json_object(&mut self, id: ETypetId, data: JsonValue) -> EDataType {
        self.types.insert(*id.raw(), RegistryItem::Raw(data));
        EDataType::Object { ident: id }
    }

    fn fetch_or_deserialize(&mut self, id: ETypetId) -> anyhow::Result<&EObjectType> {
        let data = self
            .types
            .get_mut(id.raw())
            .with_context(|| format!("Type `{id}` is not defined"))?;

        match data {
            RegistryItem::Ready(_) => {
                return Ok(self
                    .types
                    .get(id.raw())
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
        self.types.insert(*id.raw(), ready);
        Ok(self
            .types
            .get(id.raw())
            .expect("Item should be present")
            .expect_ready())
    }

    fn deserialize_all(mut self) -> anyhow::Result<Self> {
        let keys = self.types.keys().copied().collect_vec();
        for id in keys {
            self.fetch_or_deserialize(ETypetId(id))?;
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
    fn assert_defined(&self, id: &ETypetId) -> anyhow::Result<()> {
        if !self.types.contains_key(id.raw()) {
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ETypetId(Ustr);

impl ETypetId {
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

        Ok(ETypetId(data.into()))
    }

    pub fn from_path(path: &Utf8Path, types_root: &Utf8Path) -> anyhow::Result<Self> {
        let sub_path = path
            .strip_prefix(types_root)
            .map_err(|_| {
                anyhow!("Thing is outside of types root folder.\nThing: `{path}`")
            })?
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

    #[inline(always)]
    pub fn raw(&self) -> &Ustr {
        &self.0
    }
}

impl Display for ETypetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::ETypetId;
    use rstest::rstest;

    #[rstest]
    #[case("namespace:id")]
    #[case("namespace_123:a1/a2/a3/4/5/6")]
    #[case("eh:objects/faction")]
    fn should_parse_type_id(#[case] id: &str) {
        assert!(ETypetId::parse(id).is_ok())
    }

    #[test]
    fn should_fail_empty() {
        assert!(ETypetId::parse("").is_err())
    }

    #[test]
    fn should_fail_no_colon() {
        assert!(ETypetId::parse("some_name").is_err())
    }

    #[test]
    fn should_fail_empty_namespace() {
        assert!(ETypetId::parse(":some_name").is_err())
    }

    #[test]
    fn should_fail_empty_path() {
        assert!(ETypetId::parse("some_name:").is_err())
    }

    #[test]
    fn should_fail_slashes_in_namespace() {
        assert!(ETypetId::parse("namespace/other:path").is_err())
    }

    #[test]
    fn should_fail_capitalized() {
        assert!(ETypetId::parse("namespace/Path").is_err());
        assert!(ETypetId::parse("Namespace/path").is_err());
    }

    #[rstest]
    #[case("name space:id")]
    #[case("namespace:path.other")]
    #[case("namespace:path-other")]
    fn should_fail_invalid_characters(#[case] id: &str) {
        assert!(ETypetId::parse(id).is_err())
    }
}