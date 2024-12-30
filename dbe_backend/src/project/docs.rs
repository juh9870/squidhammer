use crate::etype::eobject::EObject;
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use ahash::AHashMap;
use camino::Utf8PathBuf;
use duplicate::duplicate_item;
use miette::bail;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::ops::{Deref, DerefMut};
use std::sync::LazyLock;
use strum::EnumIs;
use tracing::warn;
use ustr::Ustr;

/// Main structure for storing documentation
///
/// Application should generally use [Docs::Docs] variant. The [Docs::Stub]
/// variant should only be used for cases where the Docs structure is not yet
/// available.
///
/// Calling methods of a stub docs will panic in debug mode, and log a warning
/// in release mode.
#[derive(Debug, EnumIs)]
pub enum Docs {
    Docs(DocsContent),
    Stub,
}

static STUB_NODES: LazyLock<AHashMap<String, WithLocation<NodeDocs>>> =
    LazyLock::new(|| Default::default());

static STUB_TYPES: LazyLock<AHashMap<ETypeId, WithLocation<TypeDocs>>> =
    LazyLock::new(|| Default::default());

#[derive(Debug, Default)]
pub struct DocsContent {
    nodes: AHashMap<String, WithLocation<NodeDocs>>,
    types: AHashMap<ETypeId, WithLocation<TypeDocs>>,
}

impl Docs {
    pub fn add_file(&mut self, mut file: DocsFile, location: Utf8PathBuf) -> miette::Result<()> {
        let Docs::Docs(docs) = self else {
            panic!("Cannot add file to a stub docs");
        };
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
            match docs.nodes.entry(name) {
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
            match docs.types.entry(name) {
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

    pub fn all_nodes(&self) -> impl Iterator<Item = (&str, &NodeDocs)> {
        match self {
            Docs::Docs(docs) => {
                inform_stub("nodes");
                docs.nodes.iter()
            }
            Docs::Stub => STUB_NODES.iter(),
        }
        .map(|(k, v)| (k.as_str(), &v.value))
    }

    pub fn all_types(&self) -> impl Iterator<Item = (ETypeId, &TypeDocs)> {
        match self {
            Docs::Docs(docs) => {
                inform_stub("types");
                docs.types.iter()
            }
            Docs::Stub => STUB_TYPES.iter(),
        }
        .map(|(k, v)| (*k, &v.value))
    }

    pub fn get_node(&self, name: &str) -> Option<&NodeDocs> {
        let Docs::Docs(docs) = self else {
            inform_stub("node");
            return None;
        };
        docs.nodes.get(name).map(|n| &n.value)
    }

    pub fn get_type(&self, ty: &ETypeId) -> Option<&TypeDocs> {
        let Docs::Docs(docs) = self else {
            inform_stub("type");
            return None;
        };
        // first try getting as-is, then without generics
        docs.types
            .get(ty)
            .map(|t| &t.value)
            .or_else(|| docs.types.get(&ty.strip_generics()).map(|t| &t.value))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocsFile {
    #[serde(default)]
    pub nodes: AHashMap<String, NodeDocs>,
    #[serde(default)]
    pub types: AHashMap<ETypeId, TypeDocs>,
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
            for field in &mut ty.fields {
                validate_nonempty(&mut field.id, "field id")?;
                validate_dd(field)?;
            }
            for variant in &mut ty.variants {
                validate_nonempty(&mut variant.id, "variant id")?;
                validate_dd(variant)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeDocs {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub docs: String,
    pub inputs: Vec<NodeIODocs>,
    pub outputs: Vec<NodeIODocs>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeIODocs {
    pub title: String,
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub docs: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeDocs {
    pub description: String,
    #[serde(default)]
    pub docs: String,
    #[serde(default)]
    pub fields: Vec<FieldDocs>,
    #[serde(default)]
    pub variants: Vec<VariantDocs>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDocs {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub docs: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantDocs {
    pub id: String,
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

pub trait DocsTitled {
    fn title(&self) -> &str;
    fn title_mut(&mut self) -> &mut String;
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DocsWindowRef {
    Node(Ustr),
    Type(ETypeId),
}

impl DocsWindowRef {
    pub fn title<'docs>(&self, docs: &'docs Docs, registry: &ETypesRegistry) -> Cow<'docs, str> {
        match self {
            DocsWindowRef::Node(node) => docs
                .get_node(node.as_str())
                .map(|d| d.title.as_str())
                .map(Cow::Borrowed)
                .unwrap_or_else(|| Cow::Borrowed(node.as_str())),
            DocsWindowRef::Type(ty) => registry
                .get_object(ty)
                .map(|ty| Cow::Owned(ty.title(registry)))
                .unwrap_or_else(|| Cow::Owned(ty.to_string())),
        }
    }

    pub fn description<'docs>(&self, docs: &'docs Docs) -> Option<&'docs str> {
        match self {
            DocsWindowRef::Node(node) => {
                docs.get_node(node.as_str()).map(|d| d.description.as_str())
            }
            DocsWindowRef::Type(ty) => docs.get_type(ty).map(|d| d.description.as_str()),
        }
    }

    pub fn has_docs(&self, docs: &Docs) -> bool {
        match self {
            DocsWindowRef::Node(node) => docs.get_node(node.as_str()).is_some(),
            DocsWindowRef::Type(ty) => docs.get_type(ty).is_some(),
        }
    }
}

#[derive(Debug, Clone, EnumIs)]
pub enum DocsRef {
    NodeInput(Ustr, Ustr),
    NodeOutput(Ustr, Ustr),
    TypeField(ETypeId, Ustr),
    EnumVariant(ETypeId, Ustr),
    Custom(Cow<'static, str>),
    None,
}

impl DocsRef {
    pub fn has_field_structure(&self) -> bool {
        match self {
            DocsRef::NodeInput(_, _)
            | DocsRef::NodeOutput(_, _)
            | DocsRef::TypeField(_, _)
            | DocsRef::EnumVariant(_, _) => true,
            DocsRef::Custom(_) | DocsRef::None => false,
        }
    }

    pub fn get_description<'docs>(&self, docs: &'docs Docs) -> Option<&'docs str> {
        match self {
            DocsRef::NodeInput(node, input) => docs
                .get_node(node.as_str())
                .and_then(|d| d.inputs.iter().find(|i| i.id == input.as_str()))
                .map(|i| i.description.as_str()),
            DocsRef::NodeOutput(node, output) => docs
                .get_node(node.as_str())
                .and_then(|d| d.outputs.iter().find(|i| i.id == output.as_str()))
                .map(|o| o.description.as_str()),
            DocsRef::TypeField(ty, field) => docs
                .get_type(ty)
                .and_then(|d| d.fields.iter().find(|i| i.id == field.as_str()))
                .map(|f| f.description.as_str()),
            DocsRef::EnumVariant(ty, variant) => docs
                .get_type(ty)
                .and_then(|d| d.variants.iter().find(|i| i.id == variant.as_str()))
                .map(|f| f.description.as_str()),

            DocsRef::Custom(_) | DocsRef::None => {
                panic!("{:?} doesn't have a field structure", self)
            }
        }
    }

    pub fn get_parent_title<'b>(&self, docs: &'b Docs, registry: &ETypesRegistry) -> Cow<'b, str> {
        match self {
            DocsRef::NodeInput(node, _) | DocsRef::NodeOutput(node, _) => {
                DocsWindowRef::Node(*node).title(docs, registry)
            }
            DocsRef::TypeField(ty, _) | DocsRef::EnumVariant(ty, _) => {
                DocsWindowRef::Type(*ty).title(docs, registry)
            }
            DocsRef::Custom(_) | DocsRef::None => {
                panic!("{:?} doesn't have a field structure", self)
            }
        }
    }

    pub fn get_field_title<'docs>(&self, docs: &'docs Docs) -> Cow<'docs, str> {
        match self {
            DocsRef::NodeInput(node, input) => docs
                .get_node(node.as_str())
                .and_then(|d| d.inputs.iter().find(|i| i.id == input.as_str()))
                .map(|i| i.title.as_str())
                .unwrap_or_else(|| input.as_str())
                .into(),
            DocsRef::NodeOutput(node, output) => docs
                .get_node(node.as_str())
                .and_then(|d| d.outputs.iter().find(|i| i.id == output.as_str()))
                .map(|o| o.title.as_str())
                .unwrap_or_else(|| output.as_str())
                .into(),
            DocsRef::TypeField(_, field) => field.as_str().into(),
            DocsRef::EnumVariant(_, variant) => variant.as_str().into(),

            DocsRef::Custom(_) | DocsRef::None => {
                panic!("{:?} doesn't have a field structure", self)
            }
        }
    }

    pub fn as_window_ref(&self) -> Option<DocsWindowRef> {
        match self {
            DocsRef::NodeInput(node, _) | DocsRef::NodeOutput(node, _) => {
                Some(DocsWindowRef::Node(*node))
            }
            DocsRef::TypeField(ty, _) | DocsRef::EnumVariant(ty, _) => {
                Some(DocsWindowRef::Type(*ty))
            }
            DocsRef::Custom(_) | DocsRef::None => None,
        }
    }
}

/// Informs about usage of a stub docs
///
/// # Panics
/// Panics in debug mode, logs a warning in release mode
fn inform_stub(what: &str) {
    if cfg!(debug_assertions) {
        panic!("Cannot get {} from a stub docs", what);
    } else {
        warn!("Cannot get {} from a stub docs", what);
    }
}
