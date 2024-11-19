use crate::value::EValue;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub enum DefaultEValue {
    Owned(EValue),
    Cached(Arc<EValue>),
}

impl DefaultEValue {
    pub fn into_owned(self) -> EValue {
        match self {
            DefaultEValue::Owned(value) => value,
            DefaultEValue::Cached(value) => value.deref().clone(),
        }
    }
}

impl Deref for DefaultEValue {
    type Target = EValue;

    fn deref(&self) -> &Self::Target {
        match self {
            DefaultEValue::Owned(value) => value,
            DefaultEValue::Cached(value) => value,
        }
    }
}

impl AsRef<EValue> for DefaultEValue {
    fn as_ref(&self) -> &EValue {
        self.deref()
    }
}

impl From<EValue> for DefaultEValue {
    fn from(value: EValue) -> Self {
        Self::Owned(value)
    }
}

impl From<Arc<EValue>> for DefaultEValue {
    fn from(value: Arc<EValue>) -> Self {
        Self::Cached(value)
    }
}
