use crate::etype::econst::ETypeConst;
use atomic_refcell::AtomicRefCell;
use miette::IntoDiagnostic;
use runtime_format::ParsedFmt;
use std::collections::hash_map::Entry;
use std::ops::Deref;
use std::sync::LazyLock;
use ustr::{Ustr, UstrMap};

static FMTS: LazyLock<AtomicRefCell<UstrMap<&'static ParsedFmt<'static>>>> =
    LazyLock::new(|| AtomicRefCell::new(UstrMap::default()));

#[derive(Debug)]
pub struct ParsedFmtProp(pub &'static ParsedFmt<'static>);

impl Deref for ParsedFmtProp {
    type Target = ParsedFmt<'static>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl TryFrom<ETypeConst> for ParsedFmtProp {
    type Error = miette::Error;

    fn try_from(value: ETypeConst) -> Result<Self, Self::Error> {
        let str = Ustr::try_from(value)?;

        let borrow = FMTS.borrow();
        if let Some(fmt) = borrow.get(&str) {
            return Ok(Self(fmt));
        }

        drop(borrow);

        // Use entry instead of inserting to avoid the situation where two
        // threads try to insert the same key, since the sync point is after
        // the initial get
        match FMTS.borrow_mut().entry(str) {
            Entry::Occupied(e) => Ok(Self(e.get())),
            Entry::Vacant(e) => {
                let fmt = ParsedFmt::new(str.as_str()).into_diagnostic()?;
                // LEAK: We leak the format string, since it's stored in a static map anyway
                let fmt_ref = Box::leak(Box::new(fmt));
                e.insert(fmt_ref);
                Ok(Self(fmt_ref))
            }
        }
    }
}
