use crate::etype::econst::ETypeConst;
use crate::etype::eenum::EEnumData;
use crate::etype::eitem::EItemType;
use crate::etype::estruct::EStructData;
use crate::etype::EDataType;
use crate::json_utils::repr::{JsonRepr, Repr};
use crate::json_utils::JsonValue;
use crate::serialization::deserialize_etype;
use crate::value::id::{EListId, EMapId, ETypeId};
use crate::value::EValue;
use ahash::AHashMap;
use itertools::Itertools;
use miette::{bail, miette, Context};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use ustr::{Ustr, UstrMap};

#[derive(Debug, Clone)]
pub struct ListData {
    pub value_type: EDataType,
}

#[derive(Debug, Clone)]
pub struct MapData {
    pub key_type: EDataType,
    pub value_type: EDataType,
}

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

    pub fn parse_json(
        &self,
        registry: &ETypesRegistry,
        data: &mut JsonValue,
        inline: bool,
    ) -> miette::Result<EValue> {
        let mut data_holder: Option<JsonValue> = None;

        let data = if let EObjectType::Struct(EStructData {
            repr: Some(repr), ..
        })
        | EObjectType::Enum(EEnumData {
            repr: Some(repr), ..
        }) = self
        {
            data_holder.insert(repr.from_repr(registry, data, inline)?)
        } else {
            data
        };

        match self {
            EObjectType::Struct(s) => s
                .parse_json(registry, data, inline)
                .with_context(|| format!("in struct `{}`", s.ident)),
            EObjectType::Enum(e) => e
                .parse_json(registry, data, inline)
                .with_context(|| format!("in enum `{}`", e.ident)),
        }
    }

    pub fn extra_properties(&self) -> &AHashMap<String, ETypeConst> {
        match self {
            EObjectType::Struct(s) => &s.extra_properties,
            EObjectType::Enum(e) => &e.extra_properties,
        }
    }

    pub fn repr(&self) -> Option<&Repr> {
        match self {
            EObjectType::Struct(s) => s.repr.as_ref(),
            EObjectType::Enum(e) => e.repr.as_ref(),
        }
    }

    // pub fn default_editor(&self) -> Option<&str> {
    //     match self {
    //         EObjectType::Struct(s) => s.default_editor.as_ref().map(|e| e.as_str()),
    //         EObjectType::Enum(e) => e.default_editor.as_ref().map(|e| e.as_str()),
    //     }
    // }
    //
    // pub fn color(&self) -> Option<Color32> {
    //     match self {
    //         EObjectType::Struct(s) => s.color,
    //         EObjectType::Enum(e) => e.color,
    //     }
    // }
    //
    // pub fn port_shape(&self) -> Option<PortShape> {
    //     match self {
    //         EObjectType::Struct(s) => s.port_shape,
    //         EObjectType::Enum(e) => e.port_shape,
    //     }
    // }
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
    types: BTreeMap<ETypeId, RegistryItem>,
    lists: BTreeMap<EListId, ListData>,
    maps: BTreeMap<EMapId, MapData>,
    // editors: AHashMap<String, Box<dyn EFieldEditorConstructor>>,
    last_id: u64,
}

impl ETypesRegistry {
    pub fn from_raws(data: impl IntoIterator<Item = (ETypeId, String)>) -> miette::Result<Self> {
        let iter = data.into_iter();

        let types: BTreeMap<ETypeId, RegistryItem> = iter
            .map(|(id, v)| {
                Result::<(ETypeId, RegistryItem), miette::Error>::Ok((id, RegistryItem::Raw(v)))
            })
            .try_collect()
            .context("While grouping entries")?;

        let reg = Self {
            types,
            lists: Default::default(),
            maps: Default::default(),
            // editors: default_editors().into_iter().collect(),
            last_id: 0,
        };

        reg.deserialize_all().context("failed to deserialize types")
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
            let id = match e {
                EObjectType::Struct(s) => s.ident,
                EObjectType::Enum(e) => e.ident,
            };
            if let Some(name) = id.as_raw() {
                return name.contains(&query);
            }
            false
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

    pub fn get_list(&self, id: &EListId) -> Option<&ListData> {
        self.lists.get(id)
    }

    pub fn get_map(&self, id: &EMapId) -> Option<&MapData> {
        self.maps.get(id)
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

    pub fn register_list(&mut self, value_type: EDataType) -> EDataType {
        let id = format!("List<Item={}>", value_type.name());
        let id = EListId::from_raw(id.into());
        match self.lists.entry(id) {
            Entry::Occupied(_) => {}
            Entry::Vacant(entry) => {
                entry.insert(ListData { value_type });
            }
        }
        EDataType::List { id }
    }

    pub fn register_map(&mut self, key_type: EDataType, value_type: EDataType) -> EDataType {
        let id = format!("Map<Key={}, Item={}>", key_type.name(), value_type.name());
        let id = EMapId::from_raw(id.into());
        match self.maps.entry(id) {
            Entry::Occupied(_) => {}
            Entry::Vacant(entry) => {
                entry.insert(MapData {
                    key_type,
                    value_type,
                });
            }
        }
        EDataType::Map { id }
    }

    pub fn make_generic(
        &mut self,
        id: ETypeId,
        arguments: UstrMap<EItemType>,
    ) -> miette::Result<ETypeId> {
        let long_id = {
            let args = arguments
                .iter()
                .map(|e| format!("{}={}", e.0, e.1.ty().name()))
                .sorted()
                .join(",");
            ETypeId::from_raw(format!("{id}<{args}>").into())
        };

        if self.types.contains_key(&long_id) {
            return Ok(long_id);
        }

        let obj = self
            .fetch_or_deserialize(id)
            .with_context(|| format!("failed to find object with id {}", id))?;

        let check_generics = |args: &[Ustr]| {
            if args.len() != arguments.len() {
                bail!(
                    "Object {id} expects {} generic arguments, but {} were provided",
                    args.len(),
                    arguments.len()
                )
            }

            Ok(())
        };

        match obj.clone() {
            EObjectType::Struct(data) => {
                check_generics(&data.generic_arguments)?;
                let obj = data.apply_generics(&arguments, long_id, self)?;
                self.register_struct(long_id, obj);
                Ok(long_id)
            }
            EObjectType::Enum(data) => {
                check_generics(&data.generic_arguments)?;
                let obj = data.apply_generics(&arguments, long_id, self)?;
                self.register_enum(long_id, obj);
                Ok(long_id)
            } // EObjectType::List(mut data) => {
              //     let args = if data.value_type.is_generic() {
              //         [Ustr::from("Item")].as_slice()
              //     } else {
              //         [].as_slice()
              //     };
              //     check_generics(args)?;
              //     let Some(ty) = arguments.get(&Ustr::from("Item")) else {
              //         bail!("Generic argument `Item` is not provided");
              //     };
              //     data.value_type = ty.clone();
              //     self.register_list(long_id, data);
              //     Ok(long_id)
              // }
              // EObjectType::Map(mut data) => {
              //     let args = if data.key_type.is_generic() {
              //         [Ustr::from("Key"), Ustr::from("Item")].as_slice()
              //     } else {
              //         [].as_slice()
              //     };
              //     check_generics(args)?;
              //     let Some(key_type) = arguments.get(&Ustr::from("Key")) else {
              //         bail!("Generic argument `Key` is not provided");
              //     };
              //     let Some(value_type) = arguments.get(&Ustr::from("Item")) else {
              //         bail!("Generic argument `Item` is not provided");
              //     };
              //     data.key_type = key_type.clone();
              //     data.value_type = value_type.clone();
              //     self.register_map(long_id, data);
              //     Ok(long_id)
              // }
        }
    }

    pub fn next_temp_id(&mut self) -> ETypeId {
        self.last_id += 1;
        ETypeId::temp(self.last_id)
    }

    pub fn default_value(&self, ident: &ETypeId) -> EValue {
        let Some(data) = self.types.get(ident) else {
            return EValue::Null;
        };

        match data.expect_ready() {
            EObjectType::Struct(data) => data.default_value(self),
            EObjectType::Enum(data) => data.default_value(self),
        }
    }

    // pub fn editor_for(
    //     &self,
    //     name: Option<&str>,
    //     ty: &EItemType,
    // ) -> miette::Result<Box<dyn EFieldEditor>> {
    //     let name = match name {
    //         None => match ty {
    //             EItemType::Number(_) => "number",
    //             EItemType::String(_) => "string",
    //             EItemType::Boolean(_) => "boolean",
    //             EItemType::Const(_) => "const",
    //             EItemType::ObjectId(_) => "id",
    //             EItemType::ObjectRef(_) => "ref",
    //             EItemType::Generic(_) => "generic",
    //             EItemType::Struct(EItemStruct { id, .. }) => self
    //                 .get_object(id)
    //                 .and_then(|e| e.default_editor())
    //                 .unwrap_or("struct"),
    //             EItemType::Enum(EItemEnum { id, .. }) => self
    //                 .get_object(id)
    //                 .and_then(|e| e.default_editor())
    //                 .unwrap_or("enum"),
    //         },
    //         Some(name) => name,
    //     };
    //     let ctor = self.editors.get(name);
    //     let Some(ctor) = ctor else {
    //         bail!("Editor `{name}` is not found");
    //     };
    //
    //     ctor.make_editor(ty)
    // }

    // pub fn editor_for_or_err(&self, name: Option<&str>, ty: &EItemType) -> Box<dyn EFieldEditor> {
    //     match self.editor_for(name, ty) {
    //         Ok(editor) => editor,
    //         Err(err) => Box::new(EFieldEditorError::new(err.to_string(), ty.ty())),
    //     }
    // }

    // fn register_raw_json_object(&mut self, id: ETypetId, data: JsonValue) -> EDataType {
    //     self.types.insert(*id, RegistryItem::Raw(data));
    //     EDataType::Object { ident: id }
    // }

    pub(crate) fn fetch_or_deserialize(&mut self, id: ETypeId) -> miette::Result<&EObjectType> {
        let data = self
            .types
            .get_mut(&id)
            .ok_or_else(|| miette!("Type `{id}` is not defined"))?;

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
        let ready = RegistryItem::Ready(deserialize_etype(self, id, &old)?);
        self.types.insert(id, ready);
        Ok(self
            .types
            .get(&id)
            .expect("Item should be present")
            .expect_ready())
    }

    fn deserialize_all(mut self) -> miette::Result<Self> {
        let keys = self.types.keys().copied().collect_vec();
        for id in keys {
            self.fetch_or_deserialize(id)
                .with_context(|| format!("failed to deserialize `{id}`"))?;
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
    pub(crate) fn assert_defined(&self, id: &ETypeId) -> miette::Result<()> {
        if !self.types.contains_key(id) {
            bail!("Type `{id}` is not defined")
        }
        Ok(())
    }
}
