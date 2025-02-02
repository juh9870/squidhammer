use crate::m_try;
use crate::project::io::embedded::{walk_files, MODULES};
use crate::project::io::{sha256, ProjectIO};
use crate::project::EXTENSION_MODULE;
use include_dir::DirEntry;
use itertools::Itertools;
use miette::{bail, Context, IntoDiagnostic};
use std::borrow::Cow;
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::{error, trace};
use utils::map::dashmap::Entry;
use utils::map::DashMap;
use walkdir::WalkDir;
use zip::ZipArchive;

pub struct FilesystemIO {
    root: PathBuf,
    files: DashMap<PathBuf, FileData>,
}

impl FilesystemIO {
    pub fn new(root: PathBuf) -> miette::Result<Self> {
        let mut fs = Self {
            root,
            files: Default::default(),
        };
        fs.load_files()?;
        Ok(fs)
    }

    fn process_path(&self, file: impl AsRef<Path>) -> miette::Result<PathBuf> {
        let file = file.as_ref();
        let p = self.root.join(file);

        let abs = path_clean::clean(&p);
        // let abs = p
        //     .canonicalize()
        //     .into_diagnostic()
        //     .with_context(|| format!("failed to canonicalize path {}", p.display()))?;

        if !abs.starts_with(&self.root) {
            bail!("path `{}` is outside of the project root", p.display());
        }

        Ok(abs)
    }

    fn load_files(&mut self) -> miette::Result<()> {
        self.files.clear();

        let embedded_dir = self.root.join("\0<embedded>\0");

        for file in walk_files(&MODULES).filter_map(DirEntry::as_file) {
            let path = embedded_dir.join(file.path());
            self.files.insert(
                path,
                FileData {
                    kind: FileKind::Mem {
                        content: Cow::Borrowed(file.contents()),
                        hash: sha256(&file.contents()),
                    },
                },
            );
        }

        self.files.insert(
            embedded_dir,
            FileData {
                kind: FileKind::ReadOnlyDirectoryMarker,
            },
        );

        let wd = WalkDir::new(&self.root);
        for entry in wd {
            let entry = entry.into_diagnostic()?;
            if entry.path().is_dir() {
                continue;
            }

            let path = self.process_path(entry.path())?;

            if path.extension().and_then(|ext| ext.to_str()) == Some(EXTENSION_MODULE) {
                self.files.insert(
                    path.clone(),
                    FileData {
                        kind: FileKind::ReadOnlyDirectoryMarker,
                    },
                );
                m_try(|| {
                    let mod_archive = fs_err::read(&path)
                        .into_diagnostic()
                        .context("failed to read module archive")?;
                    let cursor = std::io::Cursor::new(mod_archive);
                    let mut archive = ZipArchive::new(cursor)
                        .into_diagnostic()
                        .context("failed to open module archive")?;

                    for i in 0..archive.len() {
                        let file = archive.by_index(i).into_diagnostic().with_context(|| {
                            format!("failed to get archive file info at index {}", i)
                        })?;
                        let unsafe_file_name = file.name().to_owned();
                        m_try(|| {
                            if file.is_dir() {
                                return Ok(());
                            }
                            if !file.is_file() {
                                bail!("symlinks are not supported in module archives");
                            }

                            let name = file
                                .enclosed_name()
                                .context("failed to get enclosed file name")?;
                            let archive_file_path = path.join(name);
                            let data = file
                                .bytes()
                                .collect::<Result<Vec<u8>, _>>()
                                .into_diagnostic()
                                .context("failed to read file")?;
                            self.files.insert(
                                archive_file_path.clone(),
                                FileData {
                                    kind: FileKind::Mem {
                                        hash: sha256(&data),
                                        content: Cow::Owned(data),
                                    },
                                },
                            );

                            Ok(())
                        })
                        .with_context(|| {
                            format!("failed to process archived file at `{}`", unsafe_file_name)
                        })?;
                    }
                    Ok(())
                })
                .with_context(|| {
                    format!("failed to read module archive at `{}`", path.display())
                })?;
            } else {
                self.files.insert(
                    path.clone(),
                    FileData {
                        kind: FileKind::Fs { hash: None },
                    },
                );
            }
        }

        Ok(())
    }
}

impl ProjectIO for FilesystemIO {
    fn list_files(&self) -> miette::Result<impl IntoIterator<Item = PathBuf> + 'static> {
        Ok(self
            .files
            .iter()
            .filter(|entry| !entry.value().is_directory())
            .map(|entry| entry.key().clone())
            .collect_vec())
    }

    fn read_file(&self, path: impl AsRef<Path>) -> miette::Result<Vec<u8>> {
        let path = self.process_path(path)?;

        if let Some(file) = self.files.get(&path) {
            if let FileKind::Mem { content, .. } = &file.kind {
                return Ok(content.to_vec());
            }
        }

        let data = fs_err::read(&path).into_diagnostic()?;

        let hash = sha256(&data);
        self.files.insert(
            path,
            FileData {
                kind: FileKind::Fs { hash: Some(hash) },
            },
        );

        Ok(data)
    }

    fn file_exists(&self, path: impl AsRef<Path>) -> miette::Result<bool> {
        let path = self.process_path(path)?;
        if self.files.contains_key(&path) {
            return Ok(true);
        }
        Ok(path.exists() && fs_err::metadata(path).into_diagnostic()?.is_file())
    }

    fn write_file(&self, path: impl AsRef<Path>, data: &[u8]) -> miette::Result<()> {
        let path = self.process_path(path)?;

        let hash = sha256(&data);
        match self.files.entry(path.clone()) {
            Entry::Occupied(mut f) => match &mut f.get_mut().kind {
                FileKind::Fs { hash: file_hash } => {
                    if file_hash.as_ref().is_some_and(|h| h == &hash) {
                        return Ok(());
                    }
                    *file_hash = Some(hash);
                }
                FileKind::Mem {
                    hash: file_hash, ..
                } => {
                    if file_hash == &hash {
                        return Ok(());
                    }
                    if cfg!(debug_assertions) {
                        bail!(
                            "attempted to save a changed mem file at `{}`",
                            path.display()
                        );
                    }
                    error!("file `{}` is not a fs file, skipping write", path.display());
                    return Ok(());
                }
                FileKind::ReadOnlyDirectoryMarker => {
                    bail!(
                        "file at `{}` is a read-only directory marker",
                        path.display()
                    );
                }
            },
            Entry::Vacant(e) => {
                e.insert(FileData {
                    kind: FileKind::Fs { hash: Some(hash) },
                });
            }
        }

        trace!("writing file {}", path.display());

        fs_err::create_dir_all(path.parent().unwrap()).into_diagnostic()?;

        fs_err::write(&path, data).into_diagnostic()?;

        Ok(())
    }

    fn delete_file(&self, path: impl AsRef<Path>) -> miette::Result<()> {
        let path = self.process_path(path)?;
        fs_err::remove_file(&path).into_diagnostic()?;

        if let Entry::Occupied(f) = self.files.entry(path.clone()) {
            if !f.get().is_writeable() {
                bail!("file `{}` is not a fs file", path.display());
            }
            f.remove();
        }

        Ok(())
    }

    fn is_file_writable(&self, path: impl AsRef<Path>) -> miette::Result<bool> {
        let path = self.process_path(path)?;

        for path in path.ancestors() {
            if let Some(file) = self.files.get(path) {
                return Ok(file.is_writeable());
            }
        }

        Ok(true)
    }

    fn flush(&mut self) -> miette::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct FileData {
    kind: FileKind,
}

impl FileData {
    fn is_writeable(&self) -> bool {
        matches!(self.kind, FileKind::Fs { .. })
    }

    fn is_directory(&self) -> bool {
        matches!(self.kind, FileKind::ReadOnlyDirectoryMarker)
    }
}

#[derive(Debug, Clone)]
enum FileKind {
    Fs {
        hash: Option<Vec<u8>>,
    },
    Mem {
        content: Cow<'static, [u8]>,
        hash: Vec<u8>,
    },
    ReadOnlyDirectoryMarker,
}
