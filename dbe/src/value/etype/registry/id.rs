use anyhow::{anyhow, bail};
use camino::Utf8Path;
use itertools::Itertools;
use serde::{Deserializer, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use ustr::Ustr;

pub fn namespace_errors(namespace: &str) -> Option<(usize, char)> {
    namespace
        .chars()
        .find_position(|c| !matches!(c, 'a'..='z' | '0'..='9' | '_'))
}
pub fn path_errors(namespace: &str) -> Option<(usize, char)> {
    namespace
        .chars()
        .find_position(|c| !matches!(c, 'a'..='z' | '0'..='9' | '_' | '/'))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum EditorId {
    Persistent(Ustr),
    Temp(u64),
}

impl serde::Serialize for EditorId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            EditorId::Persistent(id) => id.serialize(serializer),
            EditorId::Temp(id) => Err(serde::ser::Error::custom(format!(
                "temporary ETypetId can't be serialized: {}",
                id
            ))),
        }
    }
}

struct EditorIdVisitor;

impl<'de> serde::de::Visitor<'de> for EditorIdVisitor {
    type Value = EditorId;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match EditorId::parse(v) {
            Ok(data) => Ok(data),
            Err(err) => Err(serde::de::Error::custom(
                err.to_string().to_ascii_lowercase(),
            )),
        }
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
    pub fn parse(data: &str) -> anyhow::Result<Self> {
        let (namespace, path): (&str, &str) = data
            .split(':')
            .collect_tuple()
            .ok_or_else(|| anyhow!("Type path must be in a form of `namespace:path`"))?;

        if namespace.is_empty() {
            bail!("Namespace can't be empty")
        }

        if path.is_empty() {
            bail!("Path can't be empty")
        }

        if let Some((i, c)) = namespace_errors(namespace) {
            bail!("Invalid symbol `{c}` in namespace, at position {i}")
        }

        if let Some((i, c)) = path_errors(path) {
            bail!(
                "Invalid symbol `{c}` in path, at position {}",
                i + namespace.len() + 1
            )
        }

        Ok(EditorId::Persistent(data.into()))
    }

    pub fn from_path(path: &Utf8Path, types_root: &Utf8Path) -> anyhow::Result<Self> {
        let sub_path = path
            .strip_prefix(types_root)
            .map_err(|_| anyhow!("Thing is outside of types root folder.\nThing: `{path}`"))?
            .components()
            .collect_vec();
        if sub_path.len() < 2 {
            bail!("Things can't be placed in a root of types folder")
        }

        let mut segments = sub_path.into_iter();
        let namespace = segments
            .next()
            .expect("Namespace should be present")
            .to_string();

        if let Some((i, c)) = namespace_errors(&namespace) {
            bail!("Namespace folder contains invalid character `{c}` at position {i}")
        }

        let segments: Vec<String> = segments
            .with_position()
            .map(|(pos, path)| {
                let str = if matches!(pos, itertools::Position::Last | itertools::Position::Only) {
                    let p: &Utf8Path = path.as_ref();
                    p.file_stem().ok_or_else(||anyhow!("Final path segment has an empty filename"))?.to_string()
                } else {
                    path.to_string()
                };
                if let Some((i, c)) = path_errors(&str) {
                    bail!("Path folder or file contains invalid symbol `{c}` at position {i} in segment `{path}`")
                }

                Ok(str)
            })
            .try_collect()?;

        let path = segments.join("/");

        if path.is_empty() {
            bail!("Things can't be placed in a root of types folder")
        }

        Self::parse(&format!("{namespace}:{path}"))
    }
    // #[inline(always)]
    // pub fn raw(&self) -> &Ustr {
    //     &self.0
    // }
}

impl FromStr for EditorId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        EditorId::parse(s)
    }
}

impl Display for EditorId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EditorId::Persistent(id) => write!(f, "{}", id),
            EditorId::Temp(id) => write!(f, "$temp:{}", id),
        }
    }
}
