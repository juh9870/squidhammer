use crate::project::{path_has_suffix, EXTENSION_MODULE};
use crate::value::id::editor_id::Namespace;
use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DbeModule {
    pub version: semver::Version,
    pub namespace: Namespace,
    #[serde(skip)]
    pub path: Utf8PathBuf,
}

impl DbeModule {
    pub fn with_path(self, path: Utf8PathBuf) -> Self {
        Self { path, ..self }
    }
}

pub fn find_dbemodule_path(path: &Utf8Path) -> Option<&Utf8Path> {
    let mut module = None;
    for ancestor in path.ancestors() {
        if path_has_suffix(ancestor, EXTENSION_MODULE) {
            module = Some(ancestor);
        }
    }
    module
}
