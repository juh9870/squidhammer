use crate::DbeState;
use camino::Utf8PathBuf;
use egui::Ui;
use rustc_hash::FxHashMap;
use std::path::PathBuf;

pub mod init_state;
pub mod loading_state;
pub mod title_screen_state;

pub trait DbeStateHolder {
    fn update(self, ui: &mut Ui) -> DbeState;
}

#[derive(Debug)]
pub struct DbeFileSystem {
    pub root: PathBuf,
    pub raw_jsons: FxHashMap<Utf8PathBuf, String>,
    pub raw_things: FxHashMap<Utf8PathBuf, String>,
    pub raw_images: FxHashMap<Utf8PathBuf, Vec<u8>>,
}

impl DbeFileSystem {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            raw_jsons: Default::default(),
            raw_things: Default::default(),
            raw_images: Default::default(),
        }
    }
}
