use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::value::{ENumber, EValue};

pub trait EItemInfoAdapter:
    Into<EValue> + for<'a> TryFrom<&'a EValue, Error = miette::Report>
{
    fn edata_type() -> EItemInfo;
}

impl EItemInfoAdapter for ENumber {
    fn edata_type() -> EItemInfo {
        EItemInfo::simple_type(EDataType::Number)
    }
}

impl EItemInfoAdapter for bool {
    fn edata_type() -> EItemInfo {
        EItemInfo::simple_type(EDataType::Boolean)
    }
}

impl EItemInfoAdapter for String {
    fn edata_type() -> EItemInfo {
        EItemInfo::simple_type(EDataType::String)
    }
}
