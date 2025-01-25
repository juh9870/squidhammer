use crate::diagnostic::{Diagnostic, DiagnosticLevel};
use crate::path::{DiagnosticPath, DiagnosticPathSegment};
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct DiagnosticContext {
    pub diagnostics: BTreeMap<String, BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>>>,
    path: DiagnosticPath,
}

impl Default for DiagnosticContext {
    fn default() -> Self {
        DiagnosticContext {
            diagnostics: Default::default(),
            path: DiagnosticPath::empty(),
        }
    }
}

impl DiagnosticContext {
    pub fn merge(&mut self, other: DiagnosticContext) {
        for (ident, diagnostics) in other.diagnostics {
            let entry = self.diagnostics.entry(ident).or_default();
            for (path, reports) in diagnostics {
                entry.entry(path).or_default().extend(reports);
            }
        }
    }

    pub fn enter<'a>(&'a mut self, ident: &'a str) -> DiagnosticContextMut<'a> {
        let entry = self.diagnostics.entry(ident.to_string()).or_default();
        DiagnosticContextMut {
            diagnostics: entry,
            path: &mut self.path,
            ident,
            pop_on_exit: false,
        }
    }

    pub fn enter_readonly<'a>(&'a mut self, ident: &'a str) -> DiagnosticContextRef<'a> {
        let entry = self.diagnostics.entry(ident.to_string()).or_default();
        DiagnosticContextRef {
            diagnostics: entry,
            path: &mut self.path,
            ident,
            pop_on_exit: false,
        }
    }

    pub fn enter_new<'a>(&'a mut self, ident: &'a str) -> DiagnosticContextMut<'a> {
        if self.diagnostics.contains_key(ident) {
            panic!("Diagnostic context already exists for {}", ident);
        }

        self.enter(ident)
    }

    /// Checks if there are any diagnostics of the specified level or higher
    pub fn has_diagnostics(&self, level: DiagnosticLevel) -> bool {
        self.diagnostics
            .iter()
            .flat_map(|x| x.1.iter())
            .flat_map(|x| x.1.iter())
            .any(|x| x.level >= level)
    }
}

impl DiagnosticContextMut<'_> {
    pub fn emit(&mut self, info: miette::Report, level: DiagnosticLevel) {
        self.diagnostics
            .entry(self.path.clone())
            .or_default()
            .push(Diagnostic { info, level });
    }

    pub fn emit_error(&mut self, info: miette::Report) {
        self.emit(info, DiagnosticLevel::Error);
    }

    pub fn emit_warning(&mut self, info: miette::Report) {
        self.emit(info, DiagnosticLevel::Warning);
    }

    /// Clears all warnings originating from the current context or its children.
    pub fn clear_downstream(&mut self) {
        self.diagnostics
            .retain(|path, _| !path.starts_with(self.path));
    }

    /// Returns a read-only view of this context
    pub fn as_readonly(&mut self) -> DiagnosticContextRef<'_> {
        DiagnosticContextRef {
            diagnostics: self.diagnostics,
            path: &mut self.path,
            ident: self.ident,
            pop_on_exit: false,
        }
    }
}

pub type DiagnosticContextRef<'a> =
    DiagnosticContextRefHolder<'a, &'a BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>>>;
pub type DiagnosticContextMut<'a> =
    DiagnosticContextRefHolder<'a, &'a mut BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>>>;

#[derive(Debug)]
pub struct DiagnosticContextRefHolder<'a, T: 'a + ContextLike> {
    diagnostics: T,
    path: &'a mut DiagnosticPath,
    ident: &'a str,
    pop_on_exit: bool,
}

impl<'a, T: 'a + ContextLike> DiagnosticContextRefHolder<'a, T>
where
    for<'b> T::Target<'b>: ContextLike,
{
    pub fn enter(
        &mut self,
        segment: impl Into<DiagnosticPathSegment>,
    ) -> DiagnosticContextRefHolder<'_, T::Target<'_>> {
        self.path.push(segment);
        DiagnosticContextRefHolder {
            diagnostics: self.diagnostics.make_ref(),
            path: self.path,
            ident: self.ident,
            pop_on_exit: true,
        }
    }

    pub fn enter_index(&mut self, index: usize) -> DiagnosticContextRefHolder<'_, T::Target<'_>> {
        self.enter(DiagnosticPathSegment::Index(index))
    }

    pub fn enter_map_key(
        &mut self,
        key: impl Into<Cow<'static, str>>,
    ) -> DiagnosticContextRefHolder<'_, T::Target<'_>> {
        self.enter(DiagnosticPathSegment::MapKey(key.into()))
    }

    pub fn enter_field(
        &mut self,
        field: impl Into<Cow<'static, str>>,
    ) -> DiagnosticContextRefHolder<'_, T::Target<'_>> {
        self.enter(DiagnosticPathSegment::Field(field.into()))
    }

    pub fn enter_variant(
        &mut self,
        variant: impl Into<Cow<'static, str>>,
    ) -> DiagnosticContextRefHolder<'_, T::Target<'_>> {
        self.enter(DiagnosticPathSegment::Variant(variant.into()))
    }

    pub fn enter_inline(&mut self) -> DiagnosticContextRefHolder<'_, T::Target<'_>> {
        DiagnosticContextRefHolder {
            diagnostics: self.diagnostics.make_ref(),
            path: self.path,
            ident: self.ident,
            pop_on_exit: false,
        }
    }

    pub fn path(&self) -> &DiagnosticPath {
        self.path
    }

    pub fn ident(&self) -> &str {
        self.ident
    }

    pub fn full_path(&self) -> String {
        format!("{}@{}", self.ident, self.path)
    }

    /// Returns reports of the current context only.
    pub fn get_reports_shallow(&self) -> impl Iterator<Item = &Diagnostic> {
        let p = self.path();
        self.diagnostics
            .as_btreemap()
            .get(p)
            .into_iter()
            .flat_map(|v| v.iter())
    }

    /// Returns reports of the current context and all its children.
    pub fn get_reports_deep(
        &self,
    ) -> impl Iterator<Item = (&DiagnosticPath, impl IntoIterator<Item = &Diagnostic>)> {
        let p = self.path();
        self.diagnostics
            .as_btreemap()
            .range(p..)
            .take_while(|i| i.0.starts_with(p))
    }
}

impl<'a, T: 'a + ContextLike> Drop for DiagnosticContextRefHolder<'a, T> {
    fn drop(&mut self) {
        if self.pop_on_exit {
            self.path.pop();
        }
    }
}

pub trait ContextLike: sealed::Sealed {
    fn make_ref(&mut self) -> Self::Target<'_>;
    fn as_btreemap(&self) -> &BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>>;
}

impl ContextLike for &BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>> {
    fn make_ref(&mut self) -> Self::Target<'_> {
        self
    }

    fn as_btreemap(&self) -> &BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>> {
        self
    }
}

impl ContextLike for &mut BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>> {
    fn make_ref(&mut self) -> Self::Target<'_> {
        self
    }

    fn as_btreemap(&self) -> &BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>> {
        self
    }
}

mod sealed {
    use crate::diagnostic::Diagnostic;
    use crate::path::DiagnosticPath;
    use smallvec::SmallVec;
    use std::collections::BTreeMap;

    pub trait Sealed {
        type Target<'b>;
    }

    impl Sealed for &BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>> {
        type Target<'b> = &'b BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>>;
    }

    impl Sealed for &mut BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>> {
        type Target<'b> = &'b mut BTreeMap<DiagnosticPath, SmallVec<[Diagnostic; 1]>>;
    }
}
