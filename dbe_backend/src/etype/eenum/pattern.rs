use crate::etype::econst::ETypeConst;
use crate::json_utils::JsonValue;
use crate::value::id::ETypeId;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use strum::EnumIs;
use ustr::Ustr;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum EnumPattern {
    Tagged { repr: Tagged, tag: ETypeConst },
    UntaggedObject,
    Boolean,
    Number,
    String,
    Ref(ETypeId),
    Const(ETypeConst),
    List,
    Map,
    Never,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, EnumIs)]
pub enum Tagged {
    External,
    Internal {
        tag_field: Ustr,
    },
    Adjacent {
        tag_field: Ustr,
        content_field: Ustr,
    },
}

impl Display for EnumPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EnumPattern::Tagged { repr, tag } => match repr {
                Tagged::Internal { tag_field } => {
                    write!(f, "{{\"{tag_field}\": {tag}, ..}}")
                }
                Tagged::External => {
                    write!(f, "{{\"{tag}\": {{..}}}}")
                }
                Tagged::Adjacent {
                    tag_field,
                    content_field,
                } => {
                    write!(f, "{{\"{tag_field}\": {tag}, \"{content_field}\": {{..}}}}")
                }
            },
            EnumPattern::UntaggedObject => write!(f, "{{untagged}}"),
            EnumPattern::Boolean => write!(f, "{{boolean}}"),
            EnumPattern::Number => write!(f, "{{number}}"),
            EnumPattern::String => write!(f, "{{string}}"),
            EnumPattern::Const(ty) => write!(f, "{{{ty}}}"),
            EnumPattern::Ref(ty) => write!(f, "{{Ref<{ty}>}}"),
            EnumPattern::List => write!(f, "{{list}}"),
            EnumPattern::Map => write!(f, "{{map}}"),
            EnumPattern::Never => write!(f, "{{never}}"),
        }
    }
}

impl EnumPattern {
    pub fn matches_json(&self, value: &JsonValue) -> bool {
        match self {
            EnumPattern::Tagged { tag, repr } => value.as_object().is_some_and(|m| match repr {
                Tagged::External => m.get(tag.as_json_key().as_str()).is_some(),
                Tagged::Adjacent { tag_field, .. } | Tagged::Internal { tag_field } => {
                    let tag_field = m.get(tag_field.as_str());
                    tag_field.is_some_and(|val| tag.matches_json(val).by_value)
                        // When tag field is missing, match the first variant that has an empty tag
                        || (tag_field.is_none()
                            && (tag == &ETypeConst::Null
                                || tag == &ETypeConst::Number(0.into())
                                || tag == &ETypeConst::String("".into())))
                }
            }),
            EnumPattern::UntaggedObject => value.is_object(),
            EnumPattern::Boolean => value.is_boolean(),
            EnumPattern::Number => value.is_number(),
            EnumPattern::String => value.is_string(),
            EnumPattern::Ref(_) => value.is_string() || value.is_number(),
            EnumPattern::Const(c) => c.matches_json(value).by_value,
            EnumPattern::List => value.is_array(),
            EnumPattern::Map => value.is_object(),
            EnumPattern::Never => false,
        }
    }
}
