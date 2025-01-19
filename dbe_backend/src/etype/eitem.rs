use crate::etype::default::DefaultEValue;
use crate::etype::econst::ETypeConst;
use crate::etype::eobject::EObject;
use crate::etype::property::default_properties::PROP_FIELD_DEFAULT;
use crate::etype::property::FieldPropertyId;
use crate::etype::EDataType;
use crate::json_utils::repr::Repr;
use crate::registry::ETypesRegistry;
use crate::validation::Validator;
use crate::value::EValue;
use atomic_refcell::AtomicRefCell;
use std::ops::Deref;
use std::sync::{Arc, LazyLock};
use strum::EnumIs;
use tracing::error;
use ustr::Ustr;
use utils::map::HashMap;
use utils::whatever_ref::WhateverRef;

#[derive(Debug, Clone)]
pub struct EItemInfoSpecific {
    pub ty: EDataType,
    pub extra_properties: HashMap<FieldPropertyId, ETypeConst>,
    pub validators: Vec<Validator>,
}

#[derive(Debug, Clone)]
pub struct EItemInfoGeneric {
    pub argument_name: Ustr,
    pub extra_properties: HashMap<FieldPropertyId, ETypeConst>,
    pub validators: Vec<Validator>,
}

#[derive(Debug, Clone, EnumIs)]
pub enum EItemInfo {
    Specific(Arc<EItemInfoSpecific>),
    Generic(Arc<EItemInfoGeneric>),
}

impl EItemInfo {
    pub fn simple_type(ty: EDataType) -> Self {
        static CACHE: LazyLock<AtomicRefCell<HashMap<EDataType, EItemInfo>>> =
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

    /// Returns the repr for this type, if it exists
    pub fn repr<'a>(&self, registry: &'a ETypesRegistry) -> Option<impl Deref<Target = Repr> + 'a> {
        let source_ty = self.ty();
        if let EDataType::Object { ident } = source_ty {
            let obj = registry.get_object(&ident).expect("object should exist");
            if obj.repr().is_some() {
                return Some(WhateverRef::map(obj, |obj| obj.repr().unwrap()));
            }
        }

        None
    }

    pub fn default_value(&self, registry: &ETypesRegistry) -> DefaultEValue {
        match self {
            EItemInfo::Specific(ty) => {
                if let Some(value) = PROP_FIELD_DEFAULT.try_get(self.extra_properties()) {
                    let mut json_value = value.as_json_value();
                    match ty.ty.parse_json(registry, &mut json_value, false) {
                        Ok(data) => {
                            return data.into();
                        }
                        Err(err) => {
                            error!("failed to parse default value for {:?}: {}", ty.ty, err);
                        }
                    }
                }
                ty.ty.default_value(registry)
            }
            EItemInfo::Generic(ty) => {
                error!(
                    name = ty.argument_name.as_str(),
                    "generic field value was instantiated directly"
                );
                EValue::Null.into()
            }
        }
    }

    pub fn extra_properties(&self) -> &HashMap<FieldPropertyId, ETypeConst> {
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
