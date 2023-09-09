use crate::value::etype::EDataType;
use crate::value::EValue;
use anyhow::{anyhow, bail, Context};
use camino::{Utf8Path, Utf8PathBuf};
use itertools::{Itertools, Position};
use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use ustr::{Ustr, UstrMap};

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EStructField {
    pub name: Ustr,
    pub ty: EDataType,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EStructData {
    pub ident: EStructId,
    pub fields: Vec<EStructField>,
}

impl EStructData {
    pub fn new(ident: EStructId) -> EStructData {
        Self {
            fields: Default::default(),
            ident,
        }
    }
}

#[derive(Debug, Default)]
pub struct EStructRegistry {
    root: Utf8PathBuf,
    structs: UstrMap<EStructData>,
}

impl EStructRegistry {
    pub fn structs(&self) -> &UstrMap<EStructData> {
        &self.structs
    }

    pub fn register_struct(&mut self, id: EStructId, data: EStructData) -> EDataType {
        self.structs.insert(*id.raw(), data);
        EDataType::Struct { ident: id }
    }

    pub fn default_fields(&self, ident: EStructId) -> Option<UstrMap<EValue>> {
        self.structs.get(ident.raw()).map(|e| {
            e.fields
                .iter()
                .map(|f| (f.name, f.ty.default_value(self)))
                .collect()
        })
    }

    pub fn root_path(&self) -> &Utf8Path {
        self.root.as_path()
    }
}

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

#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EStructId(Ustr);

impl EStructId {
    pub fn parse(data: &str) -> anyhow::Result<Self> {
        let (namespace, path): (&str, &str) = data
            .split(":")
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

        Ok(EStructId(data.into()))
    }

    pub fn from_path(path: &Utf8Path, types_root: &Utf8Path) -> anyhow::Result<Self> {
        let sub_path = path
            .strip_prefix(types_root)
            .with_context(|| {
                format!("Thing \"{path}\" is outside of types root folder \"{types_root}\"")
            })?
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
                let str = if matches!(pos, Position::Last | Position::Only) {
                    let p: &Utf8Path = path.as_ref();
                    p.file_stem().ok_or_else(||anyhow!("Final path segment has an empty filename"))?.to_string()
                } else {
                    path.to_string()
                };
                if let Some((i, c)) = path_errors(&str) {
                    bail!("Path folder or file contains invalid symbol `{c}` at position {i} in segment \"{path}\"")
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

    pub fn raw(&self) -> &Ustr {
        &self.0
    }
}

impl Display for EStructId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
