use crate::json_utils::{json_kind, JsonValue};
use crate::value::id::editor_id::EditorId;
use miette::bail;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::{Display, Formatter};
use std::path::Path;

pub mod editor_id;

macro_rules! id_type {
    ($ident:ident) => {
        #[derive(
            Copy, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Ord, PartialOrd,
        )]
        #[serde(transparent)]
        pub struct $ident(EditorId);

        impl $ident {
            pub fn parse(data: &str) -> miette::Result<Self> {
                Ok(Self(EditorId::parse(data)?))
            }

            pub(crate) fn from_raw(raw: ustr::Ustr) -> $ident {
                Self(EditorId::Persistent(raw))
            }

            pub fn temp(id: u64) -> Self {
                Self(EditorId::Temp(id))
            }

            pub fn as_raw(&self) -> Option<&str> {
                if let EditorId::Persistent(raw) = self.0 {
                    Some(raw.as_str())
                } else {
                    None
                }
            }
        }

        impl std::fmt::Display for $ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::fmt::Debug for $ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($ident), self.0)
            }
        }

        impl std::str::FromStr for $ident {
            type Err = miette::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                $ident::parse(s)
            }
        }
    };
}

id_type!(ETypeId);

impl ETypeId {
    pub fn from_path(path: impl AsRef<Path>, types_root: impl AsRef<Path>) -> miette::Result<Self> {
        Ok(Self(EditorId::from_path(
            path.as_ref(),
            types_root.as_ref(),
        )?))
    }
}

id_type!(EValueIdStr);

id_type!(EListId);

id_type!(EMapId);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EValueId {
    String(EValueIdStr),
    Numeric(i32),
}

impl EValueId {
    pub fn parse_json(json: &mut JsonValue) -> miette::Result<EValueId> {
        match json {
            Value::Number(num) => {
                let num = num.as_f64().unwrap();
                if num < 0.0 {
                    bail!("negative numeric ID: {}", num)
                }
                if num > i32::MAX as f64 {
                    bail!(
                        "numeric ID too large, must be at most {}, but got {}",
                        i32::MAX,
                        num
                    )
                }
                Ok(EValueId::Numeric(num as i32))
            }
            Value::String(str) => Ok(EValueId::String(EValueIdStr::parse(&str)?)),
            other => {
                bail!(
                    "invalid data type. Expected string or number but got {}",
                    json_kind(&other)
                )
            }
        }
    }
}

impl Display for EValueId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EValueId::String(str) => str.fmt(f),
            EValueId::Numeric(num) => num.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ETypeId;
    use rstest::rstest;

    #[rstest]
    #[case("namespace:id")]
    #[case("namespace_123:a1/a2/a3/4/5/6")]
    #[case("eh:objects/faction")]
    fn should_parse_type_id(#[case] id: &str) {
        assert!(ETypeId::parse(id).is_ok())
    }

    #[test]
    fn should_fail_empty() {
        assert!(ETypeId::parse("").is_err())
    }

    #[test]
    fn should_fail_no_colon() {
        assert!(ETypeId::parse("some_name").is_err())
    }

    #[test]
    fn should_fail_empty_namespace() {
        assert!(ETypeId::parse(":some_name").is_err())
    }

    #[test]
    fn should_fail_empty_path() {
        assert!(ETypeId::parse("some_name:").is_err())
    }

    #[test]
    fn should_fail_slashes_in_namespace() {
        assert!(ETypeId::parse("namespace/other:path").is_err())
    }

    #[test]
    fn should_fail_capitalized() {
        assert!(ETypeId::parse("namespace/Path").is_err());
        assert!(ETypeId::parse("Namespace/path").is_err());
    }

    #[rstest]
    #[case("name space:id")]
    #[case("namespace:path.other")]
    #[case("namespace:path-other")]
    fn should_fail_invalid_characters(#[case] id: &str) {
        assert!(ETypeId::parse(id).is_err())
    }
}
