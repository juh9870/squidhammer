use crate::vfs::VfsRoot;
use crate::DbeState;
use camino::{Utf8Path, Utf8PathBuf};
use egui::Ui;
use rustc_hash::FxHashMap;

pub mod init_state;
pub mod loading_state;
pub mod title_screen_state;

pub trait DbeStateHolder {
    fn update(self, ui: &mut Ui) -> DbeState;
}

#[derive(Debug)]
pub struct DbeFileSystem {
    root: Utf8PathBuf,
    raw_files: FxHashMap<Utf8PathBuf, Vec<u8>>,
    fs: VfsRoot,
}

impl DbeFileSystem {
    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    pub fn fs(&self) -> &VfsRoot {
        &self.fs
    }

    pub fn fs_mut(&mut self) -> &mut VfsRoot {
        &mut self.fs
    }

    pub fn content(&self, path: Utf8PathBuf) -> Option<&Vec<u8>> {
        self.raw_files.get(&path)
    }
}

#[derive(Debug)]
pub struct DbeFileSystemBuilder {
    pub root: Utf8PathBuf,
    pub raw_files: FxHashMap<Utf8PathBuf, Vec<u8>>,
}

impl DbeFileSystemBuilder {
    pub fn new(root: Utf8PathBuf) -> Self {
        Self {
            root,
            raw_files: Default::default(),
        }
    }
    pub fn build(self) -> anyhow::Result<DbeFileSystem> {
        let mut fs = VfsRoot::new(self.root.clone());
        for path in self.raw_files.keys() {
            fs.create(path.clone())?;
        }
        Ok(DbeFileSystem {
            root: self.root,
            raw_files: self.raw_files,
            fs,
        })
    }
}
