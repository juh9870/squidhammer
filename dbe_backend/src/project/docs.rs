use ahash::AHashMap;
use camino::Utf8PathBuf;
use duplicate::duplicate_item;
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
    pub fn add_file(&mut self, mut file: DocsFile, location: Utf8PathBuf) -> miette::Result<()> {
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

        file.validate()?;

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

impl DocsFile {
    pub fn validate(&mut self) -> miette::Result<()> {
        for node in self.nodes.values_mut() {
            validate_nonempty(&mut node.title, "title")?;
            validate_dd(node)?;
            for input in &mut node.inputs {
                validate_nonempty(&mut input.title, "input title")?;
                validate_nonempty(&mut input.id, "input id")?;
                validate_dd(input)?;
            }
            for output in &mut node.outputs {
                validate_nonempty(&mut output.title, "output title")?;
                validate_nonempty(&mut output.id, "output id")?;
                validate_dd(output)?;
            }
        }

        for ty in self.types.values_mut() {
            validate_dd(ty)?;
            for field in ty.fields.values_mut() {
                validate_dd(field)?;
            }
            for variant in ty.variants.values_mut() {
                validate_dd(variant)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeDocs {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub docs: String,
    pub inputs: Vec<NodeIODocs>,
    pub outputs: Vec<NodeIODocs>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeIODocs {
    pub title: String,
    pub id: String,
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

fn validate_nonempty(s: &mut String, field_name: &str) -> miette::Result<()> {
    *s = s.trim().to_string();
    if s.is_empty() {
        bail!("{} cannot be empty", field_name);
    }
    Ok(())
}

fn validate_dd(docs: &mut impl DocsDescription) -> miette::Result<()> {
    validate_nonempty(docs.description_mut(), "description")?;

    let docs = docs.docs_mut();
    *docs = docs.trim().to_string();

    Ok(())
}

pub trait DocsDescription {
    fn description(&self) -> &str;
    fn docs(&self) -> &str;

    fn description_mut(&mut self) -> &mut String;
    fn docs_mut(&mut self) -> &mut String;
}

#[duplicate_item(
    ImplDocs;
    [NodeDocs];
    [NodeIODocs];
    [TypeDocs];
    [FieldDocs];
    [VariantDocs];
)]
impl DocsDescription for ImplDocs {
    fn description(&self) -> &str {
        &self.description
    }

    fn docs(&self) -> &str {
        &self.docs
    }

    fn description_mut(&mut self) -> &mut String {
        &mut self.description
    }

    fn docs_mut(&mut self) -> &mut String {
        &mut self.docs
    }
}
