use std::fmt::{Debug, Display, Formatter};
use std::path::Path;
use std::str::FromStr;

use crate::project::module::DbeModule;
use crate::project::TYPES_FOLDER;
use crate::value::id::parsing::normalize_id;
use itertools::Itertools;
use miette::{bail, miette};
use serde::{Deserializer, Serializer};
use ustr::Ustr;

#[inline(always)]
pub fn bad_namespace_char(c: char) -> bool {
    !matches!(c, 'a'..='z' | '0'..='9' | '_')
}

#[inline(always)]
pub fn bad_path_char(c: char) -> bool {
    !matches!(c, 'a'..='z' | '0'..='9' | '_' | '/')
}

pub fn namespace_errors(namespace: &str) -> Option<(usize, char)> {
    namespace.chars().find_position(|c| bad_namespace_char(*c))
}

pub fn path_errors(namespace: &str) -> Option<(usize, char)> {
    namespace.chars().find_position(|c| bad_path_char(*c))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct EditorId(pub Ustr);

impl serde::Serialize for EditorId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

struct EditorIdVisitor;

impl serde::de::Visitor<'_> for EditorIdVisitor {
    type Value = EditorId;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let id = normalize_id(v);
        Ok(EditorId(id.into()))
    }
}

impl<'de> serde::Deserialize<'de> for EditorId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(EditorIdVisitor)
    }
}

impl EditorId {
    pub fn parse(data: &str) -> miette::Result<Self> {
        let (namespace, path): (&str, &str) = data
            .split(':')
            .collect_tuple()
            .ok_or_else(|| miette!("Type path must be in a form of `namespace:path`"))?;

        let namespace = Namespace::from_str(namespace)?;

        if path.is_empty() {
            bail!("Path can't be empty")
        }

        if let Some((i, c)) = path_errors(path) {
            bail!(
                "Invalid symbol `{c}` in path, at position {}",
                i + namespace.id.len() + 1
            )
        }

        Ok(EditorId(data.into()))
    }

    pub fn from_path(module: &DbeModule, path: &Path) -> miette::Result<Self> {
        let sub_path = path
            .strip_prefix(module.path.join(TYPES_FOLDER))
            .map_err(|_| {
                miette!(
                    "Type is outside of types root folder.\nType: `{}`",
                    path.display()
                )
            })?
            .components()
            .collect_vec();
        if sub_path.is_empty() {
            bail!("Malformed type path: `{}`", path.display())
        }

        let namespace = &module.namespace;

        let segments: Vec<String> = sub_path
            .into_iter()
            .with_position()
            .map(|(pos, path)| {
                let str = if matches!(pos, itertools::Position::Last | itertools::Position::Only) {
                    let p: &Path = path.as_ref();
                    p.file_stem().ok_or_else(||miette!("Final path segment has an empty filename"))?.to_string_lossy().to_string()
                } else {
                    path.as_os_str().to_string_lossy().to_string()
                };
                if let Some((i, c)) = path_errors(&str) {
                    bail!("Path folder or file contains invalid symbol `{c}` at position {i} in segment `{}`", path.as_os_str().to_string_lossy().to_string())
                }

                Ok(str)
            })
            .try_collect()?;

        let path = segments.join("/");

        if path.is_empty() {
            bail!("Type can't be placed in a root of types folder")
        }

        Self::parse(&format!("{namespace}:{path}"))
    }
    // #[inline(always)]
    // pub fn raw(&self) -> &Ustr {
    //     &self.0
    // }
}

impl FromStr for EditorId {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        EditorId::parse(s)
    }
}

impl Display for EditorId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EditorId(id) => write!(f, "{}", id),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Namespace {
    id: String,
}

impl Debug for Namespace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.id, f)
    }
}

impl Display for Namespace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.id, f)
    }
}

impl AsRef<str> for Namespace {
    fn as_ref(&self) -> &str {
        &self.id
    }
}

impl From<Namespace> for String {
    fn from(value: Namespace) -> Self {
        value.id
    }
}

impl FromStr for Namespace {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            bail!("Namespace can't be empty")
        }

        if let Some((i, c)) = namespace_errors(s) {
            bail!("Invalid symbol `{c}` in namespace, at position {i}")
        }

        Ok(Namespace { id: s.into() })
    }
}

struct NamespaceVisitor;

impl serde::de::Visitor<'_> for NamespaceVisitor {
    type Value = Namespace;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match Namespace::from_str(v) {
            Err(e) => Err(serde::de::Error::custom(e)),
            Ok(v) => Ok(v),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Namespace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(NamespaceVisitor)
    }
}
