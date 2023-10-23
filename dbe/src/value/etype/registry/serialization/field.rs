use crate::value::etype::registry::eenum::{EEnumVariant, EnumPattern};
use crate::value::etype::registry::eitem::{
    EItemEnum, EItemObjectId, EItemStruct, EItemType, ENumberType,
};
use crate::value::etype::registry::estruct::EStructField;
use crate::value::etype::registry::{EObjectType, ETypeId, ETypesRegistry};
use crate::value::etype::ETypeConst;
use crate::value::ENumber;
use anyhow::{bail, Context};
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
    registry.assert_defined(&id)
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
        let pat = match &self {
            Self::Number(_) => EnumPattern::Number,
            Self::String(_) => EnumPattern::String,
            Self::Boolean(_) => EnumPattern::Boolean,
            Self::Const(c) => EnumPattern::Const(c.value),
            Self::Generic(_) => EnumPattern::Const(ETypeConst::Null),
            Self::Enum(_) => {
                bail!("Enum variant can't be an enum")
            }
            Self::Ref(id) => EnumPattern::Ref(id.ty),
            Self::Id(_) => {
                bail!("Object Id can't appear as an Enum variant")
            }
            Self::Struct(s) => {
                registry.assert_defined(&s.id)?;
                let target_type = registry
                    .fetch_or_deserialize(s.id)
                    .context("Error during automatic pattern key detection\n> If you see recursion error at the top of this log, consider specifying `key` parameter manually")?;

                let data = match target_type {
                    EObjectType::Enum(_) => bail!("Enum variant can't be an another enum"),
                    EObjectType::Struct(data) => data,
                };
                let pat = if s.key.is_none() {
                    let pat = data.fields.iter().filter_map(|f| {
                                match &f.ty {
                                    EItemType::Const(c) => {
                                        Some((f.name, c.value))
                                    }
                                    _ => None,
                                }
                            }).exactly_one().map_err(|_| anyhow::anyhow!("Target struct `{}` contains multiple constant fields. Please specify pattern manually", s.id))?;

                    EnumPattern::StructField(pat.0.into(), pat.1)
                } else if let Some(key) = &s.key {
                    let field =
                        data.fields
                            .iter()
                            .find(|e| e.name == s.name)
                            .with_context(|| {
                                format!(
                                    "Target struct `{}` doesn't contain a field `{}`",
                                    s.id, key,
                                )
                            })?;

                    let EItemType::Const(c) = &field.ty else {
                        bail!(
                            "Target struct `{}` contains a field `{}` but it's not a constant",
                            s.id,
                            key,
                        )
                    };

                    EnumPattern::StructField(key.as_str().into(), c.value)
                } else {
                    bail!("Multiple pattern fields are not supported")
                };

                pat
            }
        };

        let (name, item) = self.into_item(registry, root_id, path)?;
        Ok(EEnumVariant::new(name, pat, item))
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
