use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use lockfree_object_pool::{LinearObjectPool, LinearReusable};

static PATH_VEC_POOL: OnceLock<LinearObjectPool<Vec<DiagnosticPathSegment>>> = OnceLock::new();

fn path_vec_pool() -> &'static LinearObjectPool<Vec<DiagnosticPathSegment>> {
    PATH_VEC_POOL.get_or_init(|| LinearObjectPool::new(Vec::new, |v| v.clear()))
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum DiagnosticPathSegment {
    Index(usize),
    Field(Cow<'static, str>),
    Variant(Cow<'static, str>),
}

impl DiagnosticPathSegment {
    fn format(&self, prefix: bool) -> String {
        match self {
            DiagnosticPathSegment::Index(i) => {
                format!("[{i}]")
            }
            DiagnosticPathSegment::Field(f) => {
                if prefix {
                    format!(".{f}")
                } else {
                    f.to_string()
                }
            }
            DiagnosticPathSegment::Variant(v) => {
                format!("<{v}>")
            }
        }
    }

    pub fn is_index(&self, index: usize) -> bool {
        match self {
            DiagnosticPathSegment::Index(i) => *i == index,
            _ => false,
        }
    }

    pub fn is_field(&self, field: &str) -> bool {
        match self {
            DiagnosticPathSegment::Field(f) => f == field,
            _ => false,
        }
    }

    pub fn is_variant(&self, variant: &str) -> bool {
        match self {
            DiagnosticPathSegment::Variant(v) => v == variant,
            _ => false,
        }
    }
}

impl From<usize> for DiagnosticPathSegment {
    fn from(i: usize) -> Self {
        DiagnosticPathSegment::Index(i)
    }
}

impl From<&'static str> for DiagnosticPathSegment {
    fn from(f: &'static str) -> Self {
        DiagnosticPathSegment::Field(Cow::Borrowed(f))
    }
}

impl From<String> for DiagnosticPathSegment {
    fn from(f: String) -> Self {
        DiagnosticPathSegment::Field(Cow::Owned(f))
    }
}

pub struct DiagnosticPath(LinearReusable<'static, Vec<DiagnosticPathSegment>>);

impl Display for DiagnosticPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return Ok(());
        }
        write!(f, "{}", self.0[0].format(false))?;

        for segment in &self.0[1..] {
            write!(f, "{}", segment.format(true))?;
        }

        Ok(())
    }
}

impl Debug for DiagnosticPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.to_string(), f)
    }
}

impl Clone for DiagnosticPath {
    fn clone(&self) -> Self {
        let mut new_path = path_vec_pool().pull();
        new_path.extend(self.0.iter().cloned());
        DiagnosticPath(new_path)
    }
}

impl PartialEq for DiagnosticPath {
    fn eq(&self, other: &Self) -> bool {
        *self.0 == *other.0
    }
}

impl Eq for DiagnosticPath {}

impl Hash for DiagnosticPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Ord for DiagnosticPath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for DiagnosticPath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl DiagnosticPath {
    pub fn empty() -> Self {
        DiagnosticPath(path_vec_pool().pull())
    }

    pub fn push(&mut self, path: impl Into<DiagnosticPathSegment>) {
        self.0.push(path.into());
    }

    pub fn pop(&mut self) -> Option<DiagnosticPathSegment> {
        self.0.pop()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn parent(&self) -> Option<DiagnosticPath> {
        if self.0.is_empty() {
            None
        } else {
            let mut parent = self.clone();
            parent.0.pop();
            Some(parent)
        }
    }

    pub fn last(&self) -> Option<&DiagnosticPathSegment> {
        self.0.last()
    }

    pub fn last_is_index(&self, field: &str) -> bool {
        self.0
            .last()
            .map_or(false, |segment| segment.is_field(field))
    }

    pub fn last_is_field(&self, field: &str) -> bool {
        self.0
            .last()
            .map_or(false, |segment| segment.is_field(field))
    }

    pub fn last_is_variant(&self, variant: &str) -> bool {
        self.0
            .last()
            .map_or(false, |segment| segment.is_variant(variant))
    }

    pub fn extend(&mut self, mut path: DiagnosticPath) {
        self.0.append(&mut path.0);
    }

    pub fn iter(&self) -> impl Iterator<Item = &DiagnosticPathSegment> {
        self.0.iter()
    }
}

impl IntoIterator for DiagnosticPath {
    type Item = DiagnosticPathSegment;
    type IntoIter = DiagnosticPathIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        DiagnosticPathIntoIter { path: self.0 }
    }
}

pub struct DiagnosticPathIntoIter {
    path: LinearReusable<'static, Vec<DiagnosticPathSegment>>,
}

impl Iterator for DiagnosticPathIntoIter {
    type Item = DiagnosticPathSegment;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.path.remove(0))
    }
}
