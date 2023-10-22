use crate::value::etype::registry::eenum::EEnumData;
use crate::value::etype::registry::eitem::{EItemType, EItemTypeTrait};
use crate::value::etype::registry::estruct::EStructData;
use crate::value::etype::registry::serialization::deserialize_thing;
use crate::value::etype::EDataType;
use crate::value::{EValue, JsonValue};
use anyhow::{bail, Context};
use camino::{Utf8Path, Utf8PathBuf};
use egui_node_graph::DataTypeTrait;
use id::EditorId;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use ustr::{Ustr, UstrMap};

pub mod eenum;
pub mod eitem;
pub mod estruct;
pub mod id;
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

    pub fn id(&self) -> ETypeId {
        match self {
            EObjectType::Struct(s) => s.ident,
            EObjectType::Enum(e) => e.ident,
        }
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
    values: FxHashMap<EValueId, ETypeId>,
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
            values: Default::default(),
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

    pub fn all_objects_filtered(&self, search: &str) -> impl Iterator<Item = &EObjectType> {
        let query = search.to_ascii_lowercase();
        self.all_objects().filter(move |e| {
            if query.is_empty() {
                return true;
            }
            if let Some(name) = e.id().as_raw() {
                return name.contains(&query);
            }
            return false;
        })
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
        path: &str,
    ) -> anyhow::Result<ETypeId> {
        let long_id = {
            let args = arguments
                .iter()
                .map(|e| format!("{}={}", e.0, e.1.ty().name()))
                .sorted()
                .join(",");
            ETypeId::from_raw(format!("{id}<{args}>${path}").into())
        };
        if self.types.contains_key(&long_id) {
            return Ok(long_id);
        }

        let obj = self
            .fetch_or_deserialize(id)
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
                let obj = data.apply_generics(&arguments, long_id)?;
                self.register_struct(long_id, obj);
                Ok(long_id)
            }
            EObjectType::Enum(data) => {
                check_generics(&data.generic_arguments)?;
                let obj = data.apply_generics(&arguments, long_id)?;
                self.register_enum(long_id, obj);
                Ok(long_id)
            }
        }
    }

    pub fn next_temp_id(&mut self) -> ETypeId {
        self.last_id += 1;
        ETypeId::temp(self.last_id)
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

macro_rules! id_type {
    ($ident:ident) => {
        #[derive(Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
        #[serde(transparent)]
        pub struct $ident(EditorId);

        impl $ident {
            pub fn parse(data: &str) -> anyhow::Result<Self> {
                Ok(Self(EditorId::parse(data)?))
            }

            fn from_raw(raw: Ustr) -> $ident {
                Self(EditorId::Persistent(raw))
            }

            pub fn temp(id: u64) -> Self {
                Self(EditorId::Temp(id))
            }

            pub fn as_raw(&self) -> Option<&str> {
                if let EditorId::Persistent(raw) = self.0 {
                    Some(raw.as_str())
                } else {
                    None
                }
            }
        }

        impl Display for $ident {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl Debug for $ident {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($ident), self.0)
            }
        }

        impl FromStr for $ident {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                $ident::parse(s)
            }
        }
    };
}

id_type!(ETypeId);

impl ETypeId {
    pub fn from_path(path: &Utf8Path, types_root: &Utf8Path) -> anyhow::Result<Self> {
        Ok(Self(EditorId::from_path(path, types_root)?))
    }
}

id_type!(EValueId);

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
