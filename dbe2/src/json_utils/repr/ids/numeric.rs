use crate::etype::eenum::pattern::EnumPattern;
use crate::json_utils::repr::{transparent, JsonRepr};
use crate::json_utils::JsonValue;
use crate::validation::{validator_by_name, Validator};
use std::borrow::Cow;
use std::sync::LazyLock;

#[derive(Debug)]
pub struct Id;

static ID_VALIDATOR: LazyLock<Validator> =
    LazyLock::new(|| validator_by_name("ids/numeric".into()).unwrap());
static REF_VALIDATOR: LazyLock<Validator> =
    LazyLock::new(|| validator_by_name("ref/numeric".into()).unwrap());

impl JsonRepr for Id {
    transparent!("id", JsonValue::as_f64, "number", EnumPattern::Number);

    fn validators(&self) -> Cow<'static, [Validator]> {
        Cow::Owned(vec![ID_VALIDATOR.clone()])
    }
}

#[derive(Debug)]
pub struct Ref;

impl JsonRepr for Ref {
    transparent!("id", JsonValue::as_f64, "number", EnumPattern::Number);

    fn validators(&self) -> Cow<'static, [Validator]> {
        Cow::Owned(vec![REF_VALIDATOR.clone()])
    }
}
