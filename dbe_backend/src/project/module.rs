use crate::project::{path_has_suffix, EXTENSION_MODULE};
use camino::Utf8Path;

pub fn find_dbemodule_path(path: &Utf8Path) -> Option<&Utf8Path> {
    let mut module = None;
    for ancestor in path.ancestors() {
        if path_has_suffix(ancestor, EXTENSION_MODULE) {
            module = Some(ancestor);
        }
    }
    module
}
