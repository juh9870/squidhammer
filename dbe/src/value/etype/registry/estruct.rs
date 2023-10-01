use enum_dispatch::enum_dispatch;
use ustr::Ustr;

use crate::value::draw::editor::{
    BooleanEditorType, ScalarEditorType, ScalarType, StringEditorType,
};
use crate::value::etype::registry::{ETypesRegistry, ETypetId};
use crate::value::etype::{EDataType, ETypeConst};
use crate::value::{ENumber, EValue};

pub trait EStructFieldType {
    fn name(&self) -> Ustr;
    fn ty(&self) -> EDataType;
    fn default_value(&self, registry: &ETypesRegistry) -> EValue;
}

#[enum_dispatch(EStructField)]
pub(super) trait EStructFieldDependencies {
    fn check_dependencies(&self, _registry: &ETypesRegistry) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct EStructFieldScalar {
    #[knuffel(argument, str)]
    name: Ustr,
    #[knuffel(property(name = "default"))]
    default: Option<ENumber>,
    #[knuffel(property(name = "min"))]
    min: Option<ENumber>,
    #[knuffel(property(name = "max"))]
    max: Option<ENumber>,
    #[knuffel(property(name = "type"), default)]
    ty: ScalarType,
    #[knuffel(property(name = "logarithmic"))]
    logarithmic: Option<bool>,
    #[knuffel(property(name = "editor"), default)]
    editor: ScalarEditorType,
}

impl EStructFieldScalar {
    pub fn min(&self) -> Option<ENumber> {
        self.min
    }
    pub fn max(&self) -> Option<ENumber> {
        self.max
    }
    pub fn editor(&self) -> ScalarEditorType {
        self.editor
    }
    pub fn logarithmic(&self) -> Option<bool> {
        self.logarithmic
    }
    pub fn scalar_ty(&self) -> ScalarType {
        self.ty
    }
}

impl EStructFieldDependencies for EStructFieldScalar {}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct EStructFieldString {
    #[knuffel(argument, str)]
    name: Ustr,
    #[knuffel(property(name = "default"))]
    default: Option<ENumber>,
    #[knuffel(property(name = "editor"), default)]
    editor: StringEditorType,
}

impl EStructFieldString {
    pub fn editor(&self) -> StringEditorType {
        self.editor
    }
}

impl EStructFieldDependencies for EStructFieldString {}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct EStructFieldBoolean {
    #[knuffel(argument, str)]
    name: Ustr,
    #[knuffel(property(name = "default"))]
    default: Option<ENumber>,
    #[knuffel(property(name = "editor"), default)]
    editor: BooleanEditorType,
}

impl EStructFieldBoolean {
    pub fn editor(&self) -> BooleanEditorType {
        self.editor
    }
}

impl EStructFieldDependencies for EStructFieldBoolean {}

#[duplicate::duplicate_item(
    tStruct                 eType;
    [ EStructFieldScalar ]  [ Scalar ];
    [ EStructFieldString ]  [ String ];
    [ EStructFieldBoolean ] [ Boolean ];
)]
impl EStructFieldType for tStruct {
    fn name(&self) -> Ustr {
        self.name
    }

    fn ty(&self) -> EDataType {
        EDataType::eType
    }

    fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        self.default
            .map(|e| e.into())
            .unwrap_or_else(|| EDataType::eType.default_value(registry))
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct EStructFieldConst {
    #[knuffel(argument, str)]
    name: Ustr,
    #[knuffel(argument)]
    value: ETypeConst,
}

impl EStructFieldConst {
    pub fn value(&self) -> ETypeConst {
        self.value
    }
}

impl EStructFieldDependencies for EStructFieldConst {}

impl EStructFieldType for EStructFieldConst {
    fn name(&self) -> Ustr {
        self.name
    }

    fn ty(&self) -> EDataType {
        EDataType::Const { value: self.value }
    }

    fn default_value(&self, _registry: &ETypesRegistry) -> EValue {
        self.value.default_value()
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct EStructFieldStruct {
    #[knuffel(argument, str)]
    name: Ustr,
    #[knuffel(property(name = "id"), str)]
    id: ETypetId,
}

impl EStructFieldType for EStructFieldStruct {
    fn name(&self) -> Ustr {
        self.name
    }

    fn ty(&self) -> EDataType {
        EDataType::Object { ident: self.id }
    }

    fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        registry.default_value(&self.id)
    }
}

impl EStructFieldDependencies for EStructFieldStruct {
    fn check_dependencies(&self, registry: &ETypesRegistry) -> anyhow::Result<()> {
        registry.assert_defined(&self.id)
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct EStructFieldEnum {
    #[knuffel(argument, str)]
    name: Ustr,
    #[knuffel(property(name = "id"), str)]
    id: ETypetId,
}

impl EStructFieldType for EStructFieldEnum {
    fn name(&self) -> Ustr {
        self.name
    }

    fn ty(&self) -> EDataType {
        EDataType::Object { ident: self.id }
    }

    fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        registry.default_value(&self.id)
    }
}

impl EStructFieldDependencies for EStructFieldEnum {
    fn check_dependencies(&self, registry: &ETypesRegistry) -> anyhow::Result<()> {
        registry.assert_defined(&self.id)
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
#[enum_dispatch]
pub enum EStructField {
    Number(EStructFieldScalar),
    String(EStructFieldString),
    Boolean(EStructFieldBoolean),
    Const(EStructFieldConst),
    Struct(EStructFieldStruct),
    Enum(EStructFieldEnum),
}

impl EStructFieldType for EStructField {
    fn name(&self) -> Ustr {
        match self {
            EStructField::Number(f) => f.name(),
            EStructField::String(f) => f.name(),
            EStructField::Boolean(f) => f.name(),
            EStructField::Const(f) => f.name(),
            EStructField::Struct(f) => f.name(),
            EStructField::Enum(f) => f.name(),
        }
    }

    fn ty(&self) -> EDataType {
        match self {
            EStructField::Number(f) => f.ty(),
            EStructField::String(f) => f.ty(),
            EStructField::Boolean(f) => f.ty(),
            EStructField::Const(f) => f.ty(),
            EStructField::Struct(f) => f.ty(),
            EStructField::Enum(f) => f.ty(),
        }
    }

    fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        match self {
            EStructField::Number(f) => f.default_value(registry),
            EStructField::String(f) => f.default_value(registry),
            EStructField::Boolean(f) => f.default_value(registry),
            EStructField::Const(f) => f.default_value(registry),
            EStructField::Struct(f) => f.default_value(registry),
            EStructField::Enum(f) => f.default_value(registry),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EStructData {
    pub ident: ETypetId,
    pub fields: Vec<EStructField>,
}

impl EStructData {
    pub fn new(ident: ETypetId) -> EStructData {
        Self {
            fields: Default::default(),
            ident,
        }
    }

    pub fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        EValue::Struct {
            ident: self.ident,
            fields: self
                .fields
                .iter()
                .map(|f| (f.name(), f.ty().default_value(registry)))
                .collect(),
        }
    }
}
