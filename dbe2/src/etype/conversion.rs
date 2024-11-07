use crate::etype::EDataType;
use crate::value::{ENumber, EValue};

pub trait EDataTypeAdapter:
    Into<EValue> + for<'a> TryFrom<&'a EValue, Error = miette::Report>
{
    fn edata_type() -> EDataType;
}

impl EDataTypeAdapter for ENumber {
    fn edata_type() -> EDataType {
        EDataType::Number
    }
}

impl EDataTypeAdapter for bool {
    fn edata_type() -> EDataType {
        EDataType::Boolean
    }
}

impl EDataTypeAdapter for String {
    fn edata_type() -> EDataType {
        EDataType::String
    }
}
