//! A simple string formatting library. Some code is borrowed from the
//! https://crates.io/crates/runtime-format crate (MIT license).

use crate::formatting::{FmtDisplay, FormatError, FormatKeys};
use smallvec::SmallVec;
use std::cell::OnceCell;
use std::fmt::Write;

pub mod parsing;

pub mod formatting;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct PreparedFmt {
    segments: Segments,
    keys: Keys,
}

impl PreparedFmt {
    /// Get the keys used in the prepared format.
    ///
    /// Keys are listed in the order of first appearance in the format.
    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    /// Format the prepared format using the provided keys
    pub fn format_to_string(&self, keys: &impl FormatKeys) -> Result<String, FormatError> {
        let mut buf = String::new();

        let w = self.as_writer(keys);

        let _ = write!(&mut buf, "{}", w);

        w.result()?;

        Ok(buf)
    }

    /// Create a temporary writer to write the prepared format to a [Formatter]
    pub fn as_writer<'fmt, 'keys, Keys: FormatKeys>(
        &'fmt self,
        keys: &'keys Keys,
    ) -> FmtDisplay<'fmt, 'keys, Keys> {
        FmtDisplay {
            fmt: self,
            keys,
            err: OnceCell::new(),
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Segment {
    Literal(String),
    Key(String),
}

pub type Segments = SmallVec<[Segment; 4]>;
pub type Keys = SmallVec<[String; 2]>;
