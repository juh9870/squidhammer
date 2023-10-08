use crate::dbe_files::DbeFileSystem;
use crate::value::etype::registry::ETypesRegistry;

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
