use crate::etype::eenum::variant::EEnumVariantId;
use crate::value::EValue;
use smallvec::SmallVec;
use strum::EnumTryAs;
use ustr::Ustr;

pub type EditableState = SmallVec<[(Ustr, EditableStateValue); 1]>;

/// State value for editors
#[derive(Debug, Clone, EnumTryAs)]
pub enum EditableStateValue {
    /// Plain value
    Value(EValue),
    /// Variant of an enum without the data
    EnumVariant(EEnumVariantId),
}

impl From<EValue> for EditableStateValue {
    fn from(value: EValue) -> Self {
        Self::Value(value)
    }
}
