use crate::value::etype::registry::eenum::EEnumVariant;
use crate::value::etype::registry::eitem::{
    EItemEnum, EItemObjectId, EItemStruct, EItemType, ENumberType,
};
use crate::value::etype::registry::estruct::EStructField;
use crate::value::etype::registry::{ETypeId, ETypesRegistry};
use crate::value::etype::ETypeConst;
use crate::value::ENumber;
use anyhow::Context;
use itertools::Itertools;
use tracing::debug;
use ustr::Ustr;

pub(super) trait ThingFieldTrait {
    fn into_item(
        self,
        registry: &mut ETypesRegistry,
        root_id: ETypeId,
        path: &str,
    ) -> anyhow::Result<(String, EItemType)>;

    fn name(&self) -> &str;
}

fn validate_id(id: &ETypeId, registry: &ETypesRegistry) -> anyhow::Result<()> {
    registry.assert_defined(id)
}
macro_rules! impl_simple {
    ($item:ty, $field_item:tt, [$($field:ident),* $(,)?] $(, [$($reference:ident),+ $(,)?])?) => {
        paste::paste! {
            impl ThingFieldTrait for $item {
                #[allow(unused_variables)]
                fn into_item(
                    self,
                    registry: &mut ETypesRegistry,
                    _root_id: ETypeId,
                    _path: &str,
                ) -> anyhow::Result<(String, EItemType)> {
                    $($(validate_id(&self.$reference, registry)?;)*)*
                    let f = (
                        self.name,
                        EItemType::$field_item(crate::value::etype::registry::eitem::[<EItem $field_item>] {
                            $($field: self.$field),*
                        }),
                    );
                    Ok(f)
                }

                fn name(&self) -> &str {
                    &self.name
                }
            }
        }
    };
}

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldNumber {
    #[knuffel(argument)]
    name: String,
    #[knuffel(property(name = "default"))]
    default: Option<ENumber>,
    #[knuffel(property(name = "min"))]
    min: Option<ENumber>,
    #[knuffel(property(name = "max"))]
    max: Option<ENumber>,
    #[knuffel(property(name = "type"), default)]
    number_type: ENumberType,
    #[knuffel(property(name = "logarithmic"))]
    logarithmic: Option<bool>,
    #[knuffel(property(name = "editor"))]
    editor: Option<String>,
}

impl_simple!(
    FieldNumber,
    Number,
    [default, editor, logarithmic, min, max, number_type]
);

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldString {
    #[knuffel(argument)]
    name: String,
    #[knuffel(property(name = "default"))]
    default: Option<ENumber>,
    #[knuffel(property(name = "editor"))]
    editor: Option<String>,
}

impl_simple!(FieldString, String, [default, editor]);

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldBoolean {
    #[knuffel(argument)]
    name: String,
    #[knuffel(property(name = "default"))]
    default: Option<ENumber>,
    #[knuffel(property(name = "editor"))]
    editor: Option<String>,
}

impl_simple!(FieldBoolean, Boolean, [default, editor]);

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldConst {
    #[knuffel(argument)]
    name: String,
    #[knuffel(argument)]
    value: ETypeConst,
}

impl_simple!(FieldConst, Const, [value]);

fn generics(
    registry: &mut ETypesRegistry,
    mut id: ETypeId,
    root_id: ETypeId,
    generics: Vec<ThingItem>,
    path: &str,
) -> anyhow::Result<ETypeId> {
    if !generics.is_empty() {
        let gl = generics.len();
        let map: Vec<(Ustr, EItemType)> = generics
            .into_iter()
            .map(|e| {
                let p = format!("{path}::{}", e.name());
                let (name, item) = e.into_item(registry, root_id, &p)?;
                Result::<(Ustr, EItemType), anyhow::Error>::Ok((Ustr::from(name.as_str()), item))
            })
            .try_collect()
            .context("While resolving generic parameters")?;
        debug_assert_eq!(gl, map.len());
        id = registry
            .make_generic(id, (map.into_iter()).collect(), path)
            .context("While applying generic parameters")?;
        debug!("Created generic with id: {id}");
    }
    Ok(id)
}

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldStruct {
    #[knuffel(argument)]
    name: String,
    #[knuffel(argument, str)]
    id: ETypeId,
    #[knuffel(property(name = "key"), str)]
    key: Option<String>,
    #[knuffel(children)]
    generics: Vec<ThingItem>,
    #[knuffel(property(name = "editor"))]
    editor: Option<String>,
}

impl ThingFieldTrait for FieldStruct {
    fn into_item(
        self,
        registry: &mut ETypesRegistry,
        root_id: ETypeId,
        path: &str,
    ) -> anyhow::Result<(String, EItemType)> {
        let id = generics(registry, self.id, root_id, self.generics, path)?;

        validate_id(&id, registry)?;
        let f = (
            self.name,
            EItemType::Struct(EItemStruct {
                id,
                editor: self.editor,
                key: self.key,
            }),
        );
        Ok(f)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldObjectId {
    #[knuffel(argument)]
    name: String,
    #[knuffel(property(name = "editor"))]
    editor: Option<String>,
}

impl ThingFieldTrait for FieldObjectId {
    fn into_item(
        self,
        _registry: &mut ETypesRegistry,
        root_id: ETypeId,
        _path: &str,
    ) -> anyhow::Result<(String, EItemType)> {
        let f = (
            self.name,
            EItemType::ObjectId(EItemObjectId {
                ty: root_id,
                editor: self.editor,
            }),
        );
        Ok(f)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldObjectRef {
    #[knuffel(argument)]
    name: String,
    #[knuffel(argument, str)]
    ty: ETypeId,
    #[knuffel(property(name = "editor"))]
    editor: Option<String>,
}

impl_simple!(FieldObjectRef, ObjectRef, [ty, editor], [ty]);

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldEnum {
    #[knuffel(argument)]
    name: String,
    #[knuffel(argument, str)]
    id: ETypeId,
    #[knuffel(children)]
    generics: Vec<ThingItem>,
    #[knuffel(property(name = "editor"))]
    editor: Option<String>,
}
impl ThingFieldTrait for FieldEnum {
    fn into_item(
        self,
        registry: &mut ETypesRegistry,
        root_id: ETypeId,
        path: &str,
    ) -> anyhow::Result<(String, EItemType)> {
        let id = generics(registry, self.id, root_id, self.generics, path)?;

        validate_id(&id, registry)?;
        let f = (
            self.name,
            EItemType::Enum(EItemEnum {
                id,
                editor: self.editor,
            }),
        );
        Ok(f)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldGeneric {
    #[knuffel(argument)]
    name: String,
    #[knuffel(argument, str)]
    argument_name: Ustr,
}

impl_simple!(FieldGeneric, Generic, [argument_name]);

#[derive(Debug, Clone, knuffel::Decode)]
pub(super) enum ThingItem {
    Number(FieldNumber),
    String(FieldString),
    Boolean(FieldBoolean),
    Const(FieldConst),
    Struct(FieldStruct),
    Enum(FieldEnum),
    Id(FieldObjectId),
    Ref(FieldObjectRef),
    Generic(FieldGeneric),
}

impl ThingItem {
    pub fn into_enum_variant(
        self,
        registry: &mut ETypesRegistry,
        root_id: ETypeId,
        path: &str,
    ) -> anyhow::Result<EEnumVariant> {
        let (name, item) = self.into_item(registry, root_id, path)?;
        EEnumVariant::from_eitem(item, name, registry)
    }

    pub fn into_struct_field(
        self,
        registry: &mut ETypesRegistry,
        root_id: ETypeId,
        path: &str,
    ) -> anyhow::Result<EStructField> {
        let (name, item) = self.into_item(registry, root_id, path)?;
        Ok(EStructField {
            name: name.into(),
            ty: item,
        })
    }
}

impl ThingFieldTrait for ThingItem {
    fn into_item(
        self,
        registry: &mut ETypesRegistry,
        root_id: ETypeId,
        field: &str,
    ) -> anyhow::Result<(String, EItemType)> {
        match self {
            ThingItem::Number(f) => f.into_item(registry, root_id, field),
            ThingItem::String(f) => f.into_item(registry, root_id, field),
            ThingItem::Boolean(f) => f.into_item(registry, root_id, field),
            ThingItem::Const(f) => f.into_item(registry, root_id, field),
            ThingItem::Struct(f) => f.into_item(registry, root_id, field),
            ThingItem::Enum(f) => f.into_item(registry, root_id, field),
            ThingItem::Id(f) => f.into_item(registry, root_id, field),
            ThingItem::Ref(f) => f.into_item(registry, root_id, field),
            ThingItem::Generic(f) => f.into_item(registry, root_id, field),
        }
    }

    fn name(&self) -> &str {
        match self {
            ThingItem::Number(f) => f.name(),
            ThingItem::String(f) => f.name(),
            ThingItem::Boolean(f) => f.name(),
            ThingItem::Const(f) => f.name(),
            ThingItem::Struct(f) => f.name(),
            ThingItem::Enum(f) => f.name(),
            ThingItem::Id(f) => f.name(),
            ThingItem::Ref(f) => f.name(),
            ThingItem::Generic(f) => f.name(),
        }
    }
}
