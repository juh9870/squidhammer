use std::path::{Path, PathBuf};

pub use fs::FilesystemIO;

mod embedded;
mod fs;

pub trait ProjectIO: Send + Sync {
    fn list_files(&self) -> miette::Result<impl IntoIterator<Item = PathBuf> + 'static>;
    fn read_file(&self, path: impl AsRef<Path>) -> miette::Result<Vec<u8>>;
    fn file_exists(&self, path: impl AsRef<Path>) -> miette::Result<bool>;
    fn write_file(&self, path: impl AsRef<Path>, data: &[u8]) -> miette::Result<()>;
    fn delete_file(&self, path: impl AsRef<Path>) -> miette::Result<()>;

    /// Flush any pending state changes. Should be called after any calls to
    /// `read_file`, `write_file`, or `delete_file`.
    fn flush(&mut self) -> miette::Result<()>;
}

fn sha256(data: &impl AsRef<[u8]>) -> Vec<u8> {
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    sha2::Digest::update(&mut hasher, data);
    sha2::Digest::finalize(hasher).to_vec()
}
