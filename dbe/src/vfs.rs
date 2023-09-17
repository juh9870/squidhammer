use camino::{Utf8Path, Utf8PathBuf};
use std::path::StripPrefixError;
use thiserror::Error;

/// Root of the virtual file system
///
/// Only paths are stored here, actual data should be stored elsewhere, so it's
/// relatively cheap to clone, without having to worry about allocating too
/// much extra memory
#[derive(Debug, Clone)]
pub struct VfsRoot {
    root: VfsEntry,
    path: Utf8PathBuf,
}

impl VfsRoot {
    pub fn new(root: Utf8PathBuf) -> Self {
        Self {
            path: root.clone(),
            root: VfsEntry {
                ty: VfsEntryType::Directory(VfsDirectory::new(root.clone())),
                name: root
                    .file_name()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "".to_string()),
            },
        }
    }

    fn parts<'a>(
        &self,
        mut path: &'a Utf8Path,
    ) -> Result<impl IntoIterator<Item = &'a str>, VfsError> {
        if !path.is_relative() {
            path = path.strip_prefix(&self.path)?;
        }

        Ok(path)
    }

    /// Looks up and returns a reference to a file entry
    pub fn lookup(&self, path: impl AsRef<Utf8Path>) -> Result<&VfsEntry, VfsError> {
        self.root.walk(self.parts(path.as_ref())?)
    }

    /// Looks up and returns a reference to a file entry
    pub fn lookup_mut(&mut self, path: impl AsRef<Utf8Path>) -> Result<&mut VfsEntry, VfsError> {
        self.root.walk_mut(self.parts(path.as_ref())?, |_| None)
    }

    /// Creates a new directory at a given path
    pub fn mkdir(&mut self, path: impl AsRef<Utf8Path>) -> Result<&mut VfsDirectory, VfsError> {
        self.root
            .walk_mut(self.parts(path.as_ref())?, |path| {
                Some(VfsEntryType::Directory(VfsDirectory::new(path)))
            })
            .and_then(|e| e.as_directory_mut())
    }

    /// Creates a parent directory for a given path
    pub fn ensure_dir(
        &mut self,
        path: impl AsRef<Utf8Path>,
    ) -> Result<&mut VfsDirectory, VfsError> {
        let path = path.as_ref().parent().ok_or(VfsError::ParentAccess)?;
        self.mkdir(path)
    }

    pub fn create(&mut self, file: impl AsRef<Utf8Path>) -> Result<&mut VfsEntry, VfsError> {
        let path = file.as_ref();
        let dir = self.ensure_dir(path)?;
        dir.create(
            path.file_name()
                .ok_or_else(|| VfsError::EmptyFileName(path.to_path_buf()))?
                .to_string(),
            VfsEntryType::File(path.to_path_buf()),
        )
    }

    pub fn root(&self) -> &VfsEntry {
        &self.root
    }
}

#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum VfsError {
    #[error("Accessing parent entries is not supported")]
    ParentAccess,
    #[error("Entry at `{}` is not a directory", .0)]
    NotADirectory(Utf8PathBuf),
    #[error("Folder at `{}` does not contain an entry `{}`", .0, .1)]
    NotFound(Utf8PathBuf, String),
    #[error("Folder at `{}` already contain an entry `{}`", .0, .1)]
    DuplicateEntry(Utf8PathBuf, String),
    #[error("Path `{}` has an empty filename", .0)]
    EmptyFileName(Utf8PathBuf),
}

impl From<StripPrefixError> for VfsError {
    fn from(_: StripPrefixError) -> Self {
        VfsError::ParentAccess
    }
}

#[derive(Debug, Clone)]
pub enum VfsEntryType {
    File(Utf8PathBuf),
    Directory(VfsDirectory),
}

#[derive(Debug, Clone)]
pub struct VfsEntry {
    ty: VfsEntryType,
    name: String,
}

impl VfsEntry {
    pub fn path(&self) -> &Utf8Path {
        match &self.ty {
            VfsEntryType::File(path) => path,
            VfsEntryType::Directory(dir) => &dir.path,
        }
    }

    fn walk<'a>(&self, path: impl IntoIterator<Item = &'a str>) -> Result<&VfsEntry, VfsError> {
        let mut path = path.into_iter();
        match path.next() {
            None => Ok(self),
            Some(segment) => match &self.ty {
                VfsEntryType::File(path) => Err(VfsError::NotADirectory(path.clone())),
                VfsEntryType::Directory(dir) => match dir.entry(segment) {
                    None => Err(VfsError::NotFound(dir.path.clone(), segment.to_string())),
                    Some(entry) => entry.walk(path),
                },
            },
        }
    }

    fn walk_mut<'a>(
        &mut self,
        path: impl IntoIterator<Item = &'a str>,
        on_missing: impl Fn(Utf8PathBuf) -> Option<VfsEntryType>,
    ) -> Result<&mut VfsEntry, VfsError> {
        let mut path = path.into_iter();
        match path.next() {
            None => Ok(self),
            Some(segment) => match &mut self.ty {
                VfsEntryType::File(path) => Err(VfsError::NotADirectory(path.clone())),
                VfsEntryType::Directory(dir) => {
                    let segment: &str = segment;
                    let mut dir_path = None;
                    if let Some(entry) = dir.entry_or_create(segment, |dir| {
                        let data = on_missing(dir.path.join(segment));
                        if data.is_none() {
                            dir_path = Some(dir.path.clone());
                        }
                        data
                    }) {
                        return entry.walk_mut(path, on_missing);
                    }

                    Err(VfsError::NotFound(
                        dir_path.expect("Should be present in this barnch"),
                        segment.to_string(),
                    ))
                }
            },
        }
    }

    pub fn as_directory(&mut self) -> Result<&VfsDirectory, VfsError> {
        match &self.ty {
            VfsEntryType::File(path) => Err(VfsError::NotADirectory(path.clone())),
            VfsEntryType::Directory(dir) => Ok(dir),
        }
    }

    pub fn as_directory_mut(&mut self) -> Result<&mut VfsDirectory, VfsError> {
        match &mut self.ty {
            VfsEntryType::File(path) => Err(VfsError::NotADirectory(path.clone())),
            VfsEntryType::Directory(dir) => Ok(dir),
        }
    }

    pub fn ty(&self) -> &VfsEntryType {
        &self.ty
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone)]
pub struct VfsDirectory {
    children: Vec<VfsEntry>,
    path: Utf8PathBuf,
}

impl VfsDirectory {
    pub fn new(path: Utf8PathBuf) -> Self {
        Self {
            path,
            children: Default::default(),
        }
    }

    pub fn entry(&self, name: &str) -> Option<&VfsEntry> {
        self.children.iter().find(|e| e.name == name)
    }

    pub fn entry_mut(&mut self, name: &str) -> Option<&mut VfsEntry> {
        self.children.iter_mut().find(|e| e.name == name)
    }

    pub fn entry_or_create(
        &mut self,
        name: &str,
        default: impl FnOnce(&VfsDirectory) -> Option<VfsEntryType>,
    ) -> Option<&mut VfsEntry> {
        let idx = self.children.iter().position(|e| e.name == name);
        if let Some(idx) = idx {
            self.children.get_mut(idx)
        } else {
            match default(self) {
                None => None,
                Some(entry) => {
                    let entry = VfsEntry {
                        ty: entry,
                        name: name.to_string(),
                    };
                    self.children.push(entry);
                    self.children.last_mut()
                }
            }
        }
    }

    pub fn children(&self) -> impl Iterator<Item = &VfsEntry> {
        self.children.iter()
    }

    pub fn children_mut(&mut self) -> impl Iterator<Item = &mut VfsEntry> {
        self.children.iter_mut()
    }

    pub fn create(&mut self, name: String, entry: VfsEntryType) -> Result<&mut VfsEntry, VfsError> {
        if self.entry(&name).is_some() {
            return Err(VfsError::DuplicateEntry(self.path.clone(), name));
        }
        let entry = VfsEntry { ty: entry, name };
        self.children.push(entry);
        Ok(self.children.last_mut().expect("Should have children"))
    }
    
    pub fn path(&self) -> &Utf8Path {
        &self.path
    }
}
