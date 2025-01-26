use miette::{bail, IntoDiagnostic};
use std::path::{Path, PathBuf};
use tracing::trace;
use utils::map::HashMap;

pub trait ProjectIO: Send + Sync {
    fn read_file(&self, path: impl AsRef<Path>) -> miette::Result<Vec<u8>>;
    fn file_exists(&self, path: impl AsRef<Path>) -> miette::Result<bool>;
    fn write_file(&self, path: impl AsRef<Path>, data: &[u8]) -> miette::Result<()>;
    fn delete_file(&self, path: impl AsRef<Path>) -> miette::Result<()>;

    /// Flush any pending state changes. Should be called after any calls to
    /// `read_file`, `write_file`, or `delete_file`.
    fn flush(&mut self) -> miette::Result<()>;
}

pub struct FilesystemIO {
    root: PathBuf,
    file_hashes: HashMap<PathBuf, Vec<u8>>,
    hashes_chan: (
        crossbeam_channel::Sender<FileHashOp>,
        crossbeam_channel::Receiver<FileHashOp>,
    ),
}

impl FilesystemIO {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            file_hashes: Default::default(),
            hashes_chan: crossbeam_channel::unbounded(),
        }
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
}

impl ProjectIO for FilesystemIO {
    fn read_file(&self, path: impl AsRef<Path>) -> miette::Result<Vec<u8>> {
        let path = self.process_path(path)?;
        let data = fs_err::read(&path).into_diagnostic()?;

        let hash = sha256(&data);
        self.hashes_chan
            .0
            .send(FileHashOp::Set(path, hash.clone()))
            .unwrap();

        Ok(data)
    }

    fn file_exists(&self, path: impl AsRef<Path>) -> miette::Result<bool> {
        let path = self.process_path(path)?;
        Ok(path.exists() && fs_err::metadata(path).into_diagnostic()?.is_file())
    }

    fn write_file(&self, path: impl AsRef<Path>, data: &[u8]) -> miette::Result<()> {
        let path = self.process_path(path)?;
        fs_err::create_dir_all(path.parent().unwrap()).into_diagnostic()?;

        let hash = sha256(&data);

        if self.file_hashes.get(&path).is_some_and(|h| h == &hash) {
            return Ok(());
        }

        trace!("writing file {}", path.display());

        fs_err::write(&path, data).into_diagnostic()?;

        self.hashes_chan
            .0
            .send(FileHashOp::Set(path, hash.clone()))
            .unwrap();

        Ok(())
    }

    fn delete_file(&self, path: impl AsRef<Path>) -> miette::Result<()> {
        let path = self.process_path(path)?;
        fs_err::remove_file(&path).into_diagnostic()?;

        self.hashes_chan.0.send(FileHashOp::Delete(path)).unwrap();

        Ok(())
    }

    fn flush(&mut self) -> miette::Result<()> {
        for op in self.hashes_chan.1.try_iter() {
            match op {
                FileHashOp::Set(path, hash) => {
                    self.file_hashes.insert(path, hash);
                }
                FileHashOp::Delete(path) => {
                    self.file_hashes.remove(&path);
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
enum FileHashOp {
    Set(PathBuf, Vec<u8>),
    Delete(PathBuf),
}

fn sha256(data: &impl AsRef<[u8]>) -> Vec<u8> {
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    sha2::Digest::update(&mut hasher, data);
    sha2::Digest::finalize(hasher).to_vec()
}
