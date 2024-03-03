use crate::etype::econst::ETypeConst;
use crate::etype::EDataType;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use ahash::AHashMap;
use strum::EnumIs;
use tracing::error;
use ustr::Ustr;

#[derive(Debug, Clone)]
pub struct EItemTypeSpecific {
    pub ty: EDataType,
    pub extra_properties: AHashMap<String, ETypeConst>,
}

#[derive(Debug, Clone)]
pub struct EItemTypeGeneric {
    pub argument_name: Ustr,
    pub extra_properties: AHashMap<String, ETypeConst>,
}

#[derive(Debug, Clone, EnumIs)]
pub enum EItemType {
    Specific(EItemTypeSpecific),
    Generic(EItemTypeGeneric),
}

impl EItemType {
    pub fn ty(&self) -> EDataType {
        match self {
            EItemType::Specific(ty) => ty.ty,
            EItemType::Generic(ty) => {
                error!(
                    name = ty.argument_name.as_str(),
                    "Generic field type was instantiated directly",
                );
                EDataType::null()
            }
        }
    }

    pub fn default_value(&self, _registry: &ETypesRegistry) -> EValue {
        match self {
            EItemType::Specific(ty) => ty.ty.default_value(_registry),
            EItemType::Generic(ty) => {
                error!(
                    name = ty.argument_name.as_str(),
                    "Generic field value was instantiated directly"
                );
                EValue::Null
            }
        }
    }

    pub fn extra_properties(&self) -> &AHashMap<String, ETypeConst> {
        match self {
            EItemType::Specific(ty) => &ty.extra_properties,
            EItemType::Generic(ty) => &ty.extra_properties,
        }
    }
}
