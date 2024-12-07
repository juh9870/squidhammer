use crate::etype::econst::ETypeConst;
use crate::etype::property::wrappers::parser::ParsedFmtProp;
use crate::extra_properties;
use ustr::Ustr;

extra_properties! {
    pub prop<field> tag: ETypeConst;
    pub prop<field> default: ETypeConst;
    pub prop<field> inline: bool;

    /// Whether to automatically convert incoming connections to enum variants
    pub prop<object> graph_autoconvert: bool;

    /// Whether to apply implicit conversion logic when determining the variant to autoconvert to
    ///
    /// This has no effect if `graph_autoconvert` is false
    pub prop<object> graph_autoconvert_recursive: bool;

    /// The name of the variant to autoconvert to
    ///
    /// This has no effect if `graph_autoconvert` is false
    pub prop<object> graph_autoconvert_variant: Ustr;

    /// Format string for the human-readable title of the object
    pub prop<object> title: ParsedFmtProp;
}
