extern crate core;

pub mod etype;
pub mod graph;
pub mod json_utils;
pub mod project;
pub mod registry;
pub(crate) mod serialization;
pub mod validation;
pub mod value;

pub use diagnostic;

/// Helper for wrapping a code block to help with contextualizing errors
/// Better editor support but slightly worse ergonomic than a macro
#[inline(always)]
pub(crate) fn m_try<T>(func: impl FnOnce() -> miette::Result<T>) -> miette::Result<T> {
    func()
}
