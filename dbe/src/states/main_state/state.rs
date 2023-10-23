use crate::dbe_files::DbeFileSystem;
use crate::value::etype::registry::ETypesRegistry;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub(super) struct EditorData {
    pub fs: DbeFileSystem,
    pub registry: Rc<RefCell<ETypesRegistry>>,
}

impl EditorData {
    pub fn new(fs: DbeFileSystem, registry: ETypesRegistry) -> Self {
        Self {
            fs,
            registry: Rc::new(RefCell::new(registry)),
        }
    }
}
