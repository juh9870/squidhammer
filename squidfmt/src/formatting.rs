use crate::{PreparedFmt, Segment, Segments};
use std::cell::OnceCell;
use std::fmt;
use std::fmt::{Display, Formatter};
use thiserror::Error;

pub mod maps;

#[derive(Debug, Error)]
pub enum FormatKeyError {
    #[error("Unknown key")]
    UnknownKey,
    #[error("{}", .0)]
    Fmt(#[from] fmt::Error),
}

pub trait FormatKeys {
    fn fmt(&self, key: &str, f: &mut Formatter<'_>) -> Result<(), FormatKeyError>;
}

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("Failed to format key `{}`: {}", .0, .1)]
    Key(String, FormatKeyError),
    #[error("{}", .0)]
    Fmt(#[from] fmt::Error),
}

impl PreparedFmt {
    /// Format the prepared format using the provided keys
    pub fn format(&self, keys: &impl FormatKeys, f: &mut Formatter<'_>) -> Result<(), FormatError> {
        for segment in &self.segments {
            match segment {
                Segment::Literal(lit) => {
                    f.write_str(lit).map_err(FormatError::Fmt)?;
                }
                Segment::Key(key) => {
                    keys.fmt(key, f)
                        .map_err(|err| FormatError::Key(key.clone(), err))?;
                }
            }
        }
        Ok(())
    }

    /// Get the raw segments of the prepared format.
    pub fn raw_segments(&self) -> &Segments {
        &self.segments
    }
}

/// Temporary struct to allow writing [PreparedFmt] to a [Formatter]
///
/// After writing, call [FmtDisplay::result] to check for errors.
///
/// While it's possible to directly call `to_string()` on this struct, it may
/// panic if [FormatKeys] implementation returns an error.
#[derive(Debug)]
pub struct FmtDisplay<'fmt, 'keys, Keys: FormatKeys> {
    pub(crate) fmt: &'fmt PreparedFmt,
    pub(crate) keys: &'keys Keys,
    pub(crate) err: OnceCell<FormatError>,
}

impl<'fmt, 'keys, Keys: FormatKeys> FmtDisplay<'fmt, 'keys, Keys> {
    pub fn result(self) -> Result<(), FormatError> {
        match self.err.into_inner() {
            None => Ok(()),
            Some(err) => Err(err),
        }
    }
}

impl<'fmt, 'keys, Keys: FormatKeys> Display for FmtDisplay<'fmt, 'keys, Keys> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.fmt.format(self.keys, f) {
            Ok(_) => Ok(()),
            Err(err) => match err {
                FormatError::Fmt(err) => {
                    self.err
                        .set(FormatError::Fmt(err))
                        .unwrap_or_else(|_| unreachable!());
                    Err(err)
                }
                FormatError::Key(key, FormatKeyError::Fmt(err)) => {
                    self.err
                        .set(FormatError::Key(key, FormatKeyError::Fmt(err)))
                        .unwrap_or_else(|_| unreachable!());
                    Err(err)
                }
                _ => {
                    self.err.set(err).unwrap_or_else(|_| unreachable!());
                    Ok(())
                }
            },
        }
    }
}
