use crate::etype::econst::ETypeConst;
use crate::etype::eitem::EItemInfo;
use crate::etype::property::ObjectPropertyId;
use crate::json_utils::repr::Repr;
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use ahash::AHashMap;
use ustr::Ustr;

pub trait EObject {
    fn extra_properties(&self) -> &AHashMap<ObjectPropertyId, ETypeConst>;

    fn repr(&self) -> Option<&Repr>;

    fn ident(&self) -> ETypeId;

    fn generic_arguments_names(&self) -> &[Ustr];

    fn generic_arguments_values(&self) -> &[EItemInfo];

    /// Human-readable title of the object
    fn title(&self, registry: &ETypesRegistry) -> String;
}
