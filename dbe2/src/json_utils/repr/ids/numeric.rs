use crate::etype::eenum::pattern::EnumPattern;
use crate::json_utils::repr::{transparent, JsonRepr};
use crate::json_utils::JsonValue;

#[derive(Debug)]
pub struct Id;

impl JsonRepr for Id {
    transparent!("id", JsonValue::as_f64, "number", EnumPattern::Number);
}

#[derive(Debug)]
pub struct Ref;

impl JsonRepr for Ref {
    transparent!("id", JsonValue::as_f64, "number", EnumPattern::Number);
}
