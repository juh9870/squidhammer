use crate::graph::node::ports::fields::FieldMapper;
use std::fmt::Display;
use ustr::{ustr, Ustr};

pub struct SimpleMapper<Field, Local: PartialEq + Display> {
    eq: fn(&Field, &Local) -> bool,
    convert: fn(&Field) -> Local,
}

impl<Field, Local: PartialEq + Display> SimpleMapper<Field, Local> {
    pub const fn new(eq: fn(&Field, &Local) -> bool, convert: fn(&Field) -> Local) -> Self {
        Self { eq, convert }
    }
}

impl<Field, Local: PartialEq + Display> FieldMapper for SimpleMapper<Field, Local> {
    type Field = Field;
    type Local = Local;
    type Type = ();

    fn matches(&self, field: &Self::Field, local: &Self::Local) -> bool {
        (self.eq)(field, local)
    }

    fn to_local(&self, field: &Self::Field) -> Self::Local {
        (self.convert)(field)
    }
}

pub static USTR_MAPPER: SimpleMapper<String, Ustr> =
    SimpleMapper::<String, Ustr>::new(|a, b| b == a, |a| ustr(a));
