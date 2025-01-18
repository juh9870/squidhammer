use crate::formatting::{FormatKeyError, FormatKeys};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::hash::{BuildHasher, Hash};

#[allow(clippy::disallowed_types)]
impl<K, V, S> FormatKeys for std::collections::HashMap<K, V, S>
where
    K: Borrow<str> + Eq + Hash,
    V: Display,
    S: BuildHasher,
{
    fn fmt(&self, key: &str, f: &mut Formatter<'_>) -> Result<(), FormatKeyError> {
        match self.get(key) {
            Some(v) => v.fmt(f).map_err(FormatKeyError::Fmt),
            None => Err(FormatKeyError::UnknownKey),
        }
    }
}

impl<K, V> FormatKeys for BTreeMap<K, V>
where
    K: Borrow<str> + Ord,
    V: Display,
{
    fn fmt(&self, key: &str, f: &mut Formatter<'_>) -> Result<(), FormatKeyError> {
        match self.get(key) {
            Some(v) => v.fmt(f).map_err(FormatKeyError::Fmt),
            None => Err(FormatKeyError::UnknownKey),
        }
    }
}
