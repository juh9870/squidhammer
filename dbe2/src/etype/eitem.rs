use crate::etype::default::DefaultEValue;
use crate::etype::econst::ETypeConst;
use crate::etype::EDataType;
use crate::registry::ETypesRegistry;
use crate::validation::Validator;
use crate::value::EValue;
use ahash::AHashMap;
use atomic_refcell::AtomicRefCell;
use std::sync::{Arc, LazyLock};
use strum::EnumIs;
use tracing::error;
use ustr::Ustr;

#[derive(Debug, Clone)]
pub struct EItemInfoSpecific {
    pub ty: EDataType,
    pub extra_properties: AHashMap<String, ETypeConst>,
    pub validators: Vec<Validator>,
}

#[derive(Debug, Clone)]
pub struct EItemInfoGeneric {
    pub argument_name: Ustr,
    pub extra_properties: AHashMap<String, ETypeConst>,
    pub validators: Vec<Validator>,
}

#[derive(Debug, Clone, EnumIs)]
pub enum EItemInfo {
    Specific(Arc<EItemInfoSpecific>),
    Generic(Arc<EItemInfoGeneric>),
}

impl EItemInfo {
    pub fn simple_type(ty: EDataType) -> Self {
        static CACHE: LazyLock<AtomicRefCell<AHashMap<EDataType, EItemInfo>>> =
            LazyLock::new(|| AtomicRefCell::new(Default::default()));
        CACHE
            .borrow_mut()
            .entry(ty)
            .or_insert_with(|| {
                Self::Specific(Arc::new(EItemInfoSpecific {
                    ty,
                    extra_properties: Default::default(),
                    validators: Default::default(),
                }))
            })
            .clone()
    }

    pub fn ty(&self) -> EDataType {
        match self {
            EItemInfo::Specific(ty) => ty.ty,
            EItemInfo::Generic(ty) => {
                error!(
                    name = ty.argument_name.as_str(),
                    "generic field type was instantiated directly",
                );
                EDataType::null()
            }
        }
    }

    pub fn default_value(&self, registry: &ETypesRegistry) -> DefaultEValue {
        match self {
            EItemInfo::Specific(ty) => ty.ty.default_value(registry),
            EItemInfo::Generic(ty) => {
                error!(
                    name = ty.argument_name.as_str(),
                    "generic field value was instantiated directly"
                );
                EValue::Null.into()
            }
        }
    }

    pub fn extra_properties(&self) -> &AHashMap<String, ETypeConst> {
        match self {
            EItemInfo::Specific(ty) => &ty.extra_properties,
            EItemInfo::Generic(ty) => &ty.extra_properties,
        }
    }

    pub fn validators(&self) -> &Vec<Validator> {
        match self {
            EItemInfo::Specific(ty) => &ty.validators,
            EItemInfo::Generic(ty) => &ty.validators,
        }
    }
}
