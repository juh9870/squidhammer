use crate::value::id::editor_id::EditorId;
use std::path::Path;

pub mod editor_id;

macro_rules! id_type {
    ($ident:ident) => {
        #[derive(Copy, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
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

            /// Ordering for the internal usages. May change between crate versions,
            /// and should not be relied upon for any persistent store
            #[must_use]
            pub(crate) fn ord(&self) -> impl Ord {
                self.0.ord()
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
    pub fn from_path(path: &Path, types_root: &Path) -> miette::Result<Self> {
        Ok(Self(EditorId::from_path(path, types_root)?))
    }
}

id_type!(EValueId);

id_type!(EListId);

id_type!(EMapId);

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
