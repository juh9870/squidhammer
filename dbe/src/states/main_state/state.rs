use crate::dbe_files::DbeFileSystem;
use crate::value::etype::registry::ETypesRegistry;
use camino::Utf8PathBuf;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub(super) struct EditorData {
    pub fs: DbeFileSystem,
    pub registry: ETypesRegistry,
}

impl EditorData {
    pub fn new(fs: DbeFileSystem, registry: ETypesRegistry) -> Self {
        Self { fs, registry }
    }
}
