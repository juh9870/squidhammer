use crate::value::draw::editor::{
    BooleanEditorType, ENumberType, ScalarEditorType, StringEditorType,
};
use crate::value::etype::registry::{ETypeId, ETypesRegistry};
use crate::value::etype::{EDataType, ETypeConst};
use crate::value::{ENumber, EValue};
use strum_macros::AsRefStr;
use tracing::error;
use ustr::Ustr;

pub trait EItemTypeTrait {
    fn ty(&self) -> EDataType;
    fn default_value(&self, registry: &ETypesRegistry) -> EValue;
}

#[derive(Debug, Clone)]
pub struct EItemNumber {
    pub default: Option<ENumber>,
    pub min: Option<ENumber>,
    pub max: Option<ENumber>,
    pub number_type: ENumberType,
    pub logarithmic: Option<bool>,
    pub editor: ScalarEditorType,
}

#[derive(Debug, Clone)]
pub struct EItemString {
    pub default: Option<ENumber>,
    pub editor: StringEditorType,
}

#[derive(Debug, Clone)]
pub struct EItemBoolean {
    pub default: Option<ENumber>,
    pub editor: BooleanEditorType,
}

#[duplicate::duplicate_item(
tStruct          eType;
[ EItemNumber ]  [ Scalar ];
[ EItemString ]  [ String ];
[ EItemBoolean ] [ Boolean ];
)]
impl EItemTypeTrait for tStruct {
    fn ty(&self) -> EDataType {
        EDataType::eType
    }

    fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        self.default
            .map(|e| e.into())
            .unwrap_or_else(|| EDataType::eType.default_value(registry))
    }
}

#[derive(Debug, Clone)]
pub struct EItemConst {
    pub value: ETypeConst,
}

impl EItemTypeTrait for EItemConst {
    fn ty(&self) -> EDataType {
        EDataType::Const { value: self.value }
    }

    fn default_value(&self, _registry: &ETypesRegistry) -> EValue {
        self.value.default_value()
    }
}

#[derive(Debug, Clone)]
pub struct EItemStruct {
    pub id: ETypeId,
}

impl EItemTypeTrait for EItemStruct {
    fn ty(&self) -> EDataType {
        EDataType::Object { ident: self.id }
    }

    fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        registry.default_value(&self.id)
    }
}

#[derive(Debug, Clone)]
pub struct EItemEnum {
    pub id: ETypeId,
}

impl EItemTypeTrait for EItemEnum {
    fn ty(&self) -> EDataType {
        EDataType::Object { ident: self.id }
    }

    fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        registry.default_value(&self.id)
    }
}

#[derive(Debug, Clone)]
pub struct EItemObjectId {
    pub ty: ETypeId,
}

impl EItemTypeTrait for EItemObjectId {
    fn ty(&self) -> EDataType {
        EDataType::Id { ty: self.ty }
    }

    fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        EValue::Id {
            ty: self.ty,
            value: None,
        };
        registry.default_value(&self.ty)
    }
}

#[derive(Debug, Clone)]
pub struct EItemObjectRef {
    pub ty: ETypeId,
}

impl EItemTypeTrait for EItemObjectRef {
    fn ty(&self) -> EDataType {
        EDataType::Ref { ty: self.ty }
    }

    fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        EValue::Ref {
            ty: self.ty,
            value: None,
        };
        registry.default_value(&self.ty)
    }
}

#[derive(Debug, Clone)]
pub struct EItemGeneric {
    pub argument_name: Ustr,
}

impl EItemTypeTrait for EItemGeneric {
    fn ty(&self) -> EDataType {
        error!(
            name = self.argument_name.as_str(),
            "Generic field was instantiated directly",
        );
        EDataType::Const {
            value: ETypeConst::Null,
        }
    }

    fn default_value(&self, _registry: &ETypesRegistry) -> EValue {
        error!(
            name = self.argument_name.as_str(),
            "Generic field was instantiated directly"
        );
        EValue::Null
    }
}

#[derive(Debug, Clone, AsRefStr)]
pub enum EItemType {
    Number(EItemNumber),
    String(EItemString),
    Boolean(EItemBoolean),
    Const(EItemConst),
    Struct(EItemStruct),
    Enum(EItemEnum),
    ObjectId(EItemObjectId),
    ObjectRef(EItemObjectRef),
    Generic(EItemGeneric),
}

impl EItemTypeTrait for EItemType {
    fn ty(&self) -> EDataType {
        match self {
            EItemType::Number(f) => f.ty(),
            EItemType::String(f) => f.ty(),
            EItemType::Boolean(f) => f.ty(),
            EItemType::Const(f) => f.ty(),
            EItemType::Struct(f) => f.ty(),
            EItemType::Enum(f) => f.ty(),
            EItemType::Generic(f) => f.ty(),
            EItemType::ObjectId(f) => f.ty(),
            EItemType::ObjectRef(f) => f.ty(),
        }
    }

    fn default_value(&self, registry: &ETypesRegistry) -> EValue {
        match self {
            EItemType::Number(f) => f.default_value(registry),
            EItemType::String(f) => f.default_value(registry),
            EItemType::Boolean(f) => f.default_value(registry),
            EItemType::Const(f) => f.default_value(registry),
            EItemType::Struct(f) => f.default_value(registry),
            EItemType::Enum(f) => f.default_value(registry),
            EItemType::Generic(f) => f.default_value(registry),
            EItemType::ObjectId(f) => f.default_value(registry),
            EItemType::ObjectRef(f) => f.default_value(registry),
        }
    }
}
