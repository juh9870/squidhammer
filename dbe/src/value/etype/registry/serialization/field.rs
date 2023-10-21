use crate::value::draw::editor::{
    BooleanEditorType, ENumberType, ScalarEditorType, StringEditorType,
};
use crate::value::etype::registry::eenum::{EEnumVariant, EnumPattern};
use crate::value::etype::registry::eitem::EItemType;
use crate::value::etype::registry::estruct::EStructField;
use crate::value::etype::registry::{EObjectType, ETypeId, ETypesRegistry};
use crate::value::etype::ETypeConst;
use crate::value::ENumber;
use anyhow::{bail, Context};
use itertools::Itertools;
use ustr::Ustr;

pub(super) trait ThingStructItemTrait {
    fn into_item(
        self,
        registry: &mut ETypesRegistry,
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
            impl ThingStructItemTrait for $item {
                #[allow(unused_variables)]
                fn into_item(
                    self,
                    registry: &mut ETypesRegistry,
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
    #[knuffel(property(name = "editor"), default)]
    editor: ScalarEditorType,
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
    #[knuffel(property(name = "editor"), default)]
    editor: StringEditorType,
}

impl_simple!(FieldString, String, [default, editor]);

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldBoolean {
    #[knuffel(argument)]
    name: String,
    #[knuffel(property(name = "default"))]
    default: Option<ENumber>,
    #[knuffel(property(name = "editor"), default)]
    editor: BooleanEditorType,
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

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldStruct {
    #[knuffel(argument)]
    name: String,
    #[knuffel(argument, str)]
    id: ETypeId,
    #[knuffel(property(name = "key"), str)]
    key: Option<String>,
}

impl_simple!(FieldStruct, Struct, [id], [id]);

#[derive(Debug, knuffel::Decode, Clone)]
pub(super) struct FieldEnum {
    #[knuffel(argument)]
    name: String,
    #[knuffel(argument, str)]
    id: ETypeId,
}

impl_simple!(FieldEnum, Enum, [id], [id]);

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
}

impl ThingItem {
    pub fn into_enum_variant(
        self,
        registry: &mut ETypesRegistry,
        path: &str,
    ) -> anyhow::Result<EEnumVariant> {
        let pat = match &self {
            Self::Number(_) => EnumPattern::Number,
            Self::String(_) => EnumPattern::String,
            Self::Boolean(_) => EnumPattern::Boolean,
            Self::Const(c) => EnumPattern::Const(c.value),
            Self::Enum(_) => {
                bail!("Enum variant can't be an enum")
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

        let (name, item) = self.into_item(registry, path)?;
        Ok(EEnumVariant::new(name, pat, item))
    }

    pub fn into_struct_field(
        self,
        registry: &mut ETypesRegistry,
        path: &str,
    ) -> anyhow::Result<EStructField> {
        let (name, item) = self.into_item(registry, path)?;
        Ok(EStructField {
            name: name.into(),
            ty: item,
        })
    }
}

impl ThingStructItemTrait for ThingItem {
    fn into_item(
        self,
        registry: &mut ETypesRegistry,
        field: &str,
    ) -> anyhow::Result<(String, EItemType)> {
        match self {
            ThingItem::Number(f) => f.into_item(registry, field),
            ThingItem::String(f) => f.into_item(registry, field),
            ThingItem::Boolean(f) => f.into_item(registry, field),
            ThingItem::Const(f) => f.into_item(registry, field),
            ThingItem::Struct(f) => f.into_item(registry, field),
            ThingItem::Enum(f) => f.into_item(registry, field),
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
        }
    }
}
