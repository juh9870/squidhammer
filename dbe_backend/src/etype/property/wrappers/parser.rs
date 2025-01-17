use crate::etype::econst::ETypeConst;
use atomic_refcell::AtomicRefCell;
use miette::IntoDiagnostic;
use squidfmt::PreparedFmt;
use std::collections::hash_map::Entry;
use std::ops::Deref;
use std::sync::LazyLock;
use ustr::{ustr, Ustr};
use utils::map::HashMap;

static FMTS: LazyLock<AtomicRefCell<HashMap<String, &'static PreparedFmt>>> =
    LazyLock::new(|| AtomicRefCell::new(HashMap::default()));

pub fn get_formatter(str: &str) -> miette::Result<&'static PreparedFmt> {
    let borrow = FMTS.borrow();
    if let Some(fmt) = borrow.get(str) {
        return Ok(*fmt);
    }

    drop(borrow);

    let str = ustr(str).as_str();

    // Use entry instead of inserting to avoid the situation where two
    // threads try to insert the same key, since the sync point is after
    // the initial get
    match FMTS.borrow_mut().entry(str.to_string()) {
        Entry::Occupied(e) => Ok(e.get()),
        Entry::Vacant(e) => {
            let fmt = PreparedFmt::parse(str).into_diagnostic()?;
            // LEAK: We leak the formatter, since it's stored in a static map anyway
            let fmt_ref = Box::leak(Box::new(fmt));
            e.insert(fmt_ref);
            Ok(fmt_ref)
        }
    }
}

#[derive(Debug)]
pub struct ParsedFmtProp(pub &'static PreparedFmt);

impl Deref for ParsedFmtProp {
    type Target = PreparedFmt;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl TryFrom<ETypeConst> for ParsedFmtProp {
    type Error = miette::Error;

    fn try_from(value: ETypeConst) -> Result<Self, Self::Error> {
        let str = Ustr::try_from(value)?;

        get_formatter(str.as_str()).map(ParsedFmtProp)
    }
}
