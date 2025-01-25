use miette::{bail, IntoDiagnostic};
use std::path::{Path, PathBuf};

pub trait ProjectIO: Send + Sync {
    fn read_file(&self, path: impl AsRef<Path>) -> miette::Result<Vec<u8>>;
    fn file_exists(&self, path: impl AsRef<Path>) -> miette::Result<bool>;
    fn write_file(&self, path: impl AsRef<Path>, data: &[u8]) -> miette::Result<()>;
    fn delete_file(&self, path: impl AsRef<Path>) -> miette::Result<()>;
}

pub struct FilesystemIO {
    root: PathBuf,
}

impl FilesystemIO {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
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
        fs_err::read(path).into_diagnostic()
    }

    fn file_exists(&self, path: impl AsRef<Path>) -> miette::Result<bool> {
        let path = self.process_path(path)?;
        Ok(path.exists() && fs_err::metadata(path).into_diagnostic()?.is_file())
    }

    fn write_file(&self, path: impl AsRef<Path>, data: &[u8]) -> miette::Result<()> {
        let path = self.process_path(path)?;
        fs_err::create_dir_all(path.parent().unwrap()).into_diagnostic()?;
        fs_err::write(path, data).into_diagnostic()
    }

    fn delete_file(&self, path: impl AsRef<Path>) -> miette::Result<()> {
        let path = self.process_path(path)?;
        fs_err::remove_file(path).into_diagnostic()
    }
}
