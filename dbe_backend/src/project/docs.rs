use ahash::AHashMap;
use camino::Utf8PathBuf;
use miette::bail;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Default)]
pub struct Docs {
    pub nodes: AHashMap<String, WithLocation<NodeDocs>>,
    pub types: AHashMap<String, WithLocation<TypeDocs>>,
}

impl Docs {
    pub fn add_file(&mut self, file: DocsFile, location: Utf8PathBuf) -> miette::Result<()> {
        match (file.nodes.len(), file.types.len()) {
            (0, 0) => {
                // nothing to do
                return Ok(());
            }
            (1, 0) | (0, 1) => {
                // all is good
            }
            (_, 0) => {
                bail!("Only one node can be documented in a single file");
            }
            (0, _) => {
                bail!("Only one type can be documented in a single file");
            }
            _ => {
                bail!("Nodes and types cannot be documented in the same file, and only one of either can be documented in a single file");
            }
        }

        for (name, node) in file.nodes {
            match self.nodes.entry(name) {
                Entry::Vacant(e) => {
                    e.insert(WithLocation {
                        value: node,
                        location: location.clone(),
                    });
                }
                Entry::Occupied(e) => {
                    bail!(
                        "Node `{}` is already documented in `{}`",
                        e.key(),
                        e.get().location
                    );
                }
            }
        }

        for (name, ty) in file.types {
            match self.types.entry(name) {
                Entry::Vacant(e) => {
                    e.insert(WithLocation {
                        value: ty,
                        location: location.clone(),
                    });
                }
                Entry::Occupied(e) => {
                    bail!(
                        "Type `{}` is already documented in `{}`",
                        e.key(),
                        e.get().location
                    );
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocsFile {
    #[serde(default)]
    pub nodes: AHashMap<String, NodeDocs>,
    #[serde(default)]
    pub types: AHashMap<String, TypeDocs>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeDocs {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub docs: String,
    pub inputs: AHashMap<String, NodeIODocs>,
    pub outputs: AHashMap<String, NodeIODocs>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeIODocs {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub docs: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypeDocs {
    pub description: String,
    #[serde(default)]
    pub docs: String,
    pub fields: AHashMap<String, FieldDocs>,
    pub variants: AHashMap<String, VariantDocs>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldDocs {
    pub description: String,
    #[serde(default)]
    pub docs: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VariantDocs {
    pub description: String,
    #[serde(default)]
    pub docs: String,
}

#[derive(Debug)]
pub struct WithLocation<T> {
    pub value: T,
    pub location: Utf8PathBuf,
}

impl<T> Deref for WithLocation<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for WithLocation<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
