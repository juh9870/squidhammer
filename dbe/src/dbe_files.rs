use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail, Context, Error};
use camino::{Utf8Path, Utf8PathBuf};
use duplicate::duplicate_item;
use rustc_hash::FxHashMap;
use tracing::debug;

use crate::editable::EditableFile;
use utils::somehow;

use crate::value::etype::registry::ETypeId;

#[derive(Debug, Default, Clone)]
pub enum EditorItem {
    #[default]
    Empty,
    Raw(Vec<u8>),
    Value(EditableFile),
    Type(ETypeId),
}

#[duplicate_item(
method      variant     data_type;
[ raw ]     [ Raw ]     [ Vec < u8 > ];
[ type ]    [ Type ]    [ super::ETypeId ];
[ value ]   [ Value ]   [ super::EditableFile ];
)]
mod editor_item {
    impl super::EditorItem {
        paste::paste! {
            pub fn [<as_ method>](&self) -> Option<&data_type> {
                if let Self::variant(data) = self { Some(data) } else { None }
            }
            pub fn [<as_ method _mut>](&mut self) -> Option<&mut data_type> {
                if let Self::variant(data) = self { Some(data) } else { None }
            }
            pub fn [<is_ method>](&self) -> bool {
                matches!(self, Self::variant(..))
            }
        }
    }

    impl From<data_type> for super::EditorItem {
        fn from(item: data_type) -> Self {
            Self::variant(item)
        }
    }
}

impl EditorItem {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

#[derive(Debug)]
pub struct DbeFileSystem {
    root: Utf8PathBuf,
    fs: BTreeMap<Utf8PathBuf, EditorItem>,
    dirty: BTreeSet<Utf8PathBuf>,
}

impl DbeFileSystem {
    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    pub fn fs(&self) -> &BTreeMap<Utf8PathBuf, EditorItem> {
        &self.fs
    }

    pub fn fs_mut(&mut self) -> &mut BTreeMap<Utf8PathBuf, EditorItem> {
        &mut self.fs
    }

    pub fn content(&self, path: &Utf8Path) -> Option<&EditorItem> {
        self.fs.get(path)
    }

    pub fn content_mut(&mut self, path: &Utf8Path) -> Option<&mut EditorItem> {
        self.set_dirty(path.to_path_buf());
        self.fs.get_mut(path)
    }

    #[inline(always)]
    pub fn set_dirty(&mut self, path: Utf8PathBuf) {
        self.dirty.insert(path);
    }
    pub fn new_item(&mut self, item: EditorItem, path: &Utf8Path) -> anyhow::Result<()> {
        let Ok(subpath) = path.strip_prefix(&self.root) else {
            bail!("Can't create a file outside of root DBE folder")
        };
        let mut any = false;
        for component in subpath.components() {
            any = true;
            match component.as_str() {
                "." | ".." => bail!("File path must be normalized"),
                _ => {}
            }
        }

        if !any {
            bail!("File path can't be a root folder");
        }

        if self
            .fs
            .range::<Utf8PathBuf, _>(path.to_path_buf()..)
            .take_while(|e| e.0.starts_with(path))
            .next()
            .is_some()
        {
            bail!("Directory with this path already exists")
        }

        somehow!({
            self.fs.insert(path.to_path_buf(), item);
            self.set_dirty(path.to_path_buf());
        })
        .with_context(|| format!("While creating file `{path}`"))
    }

    /// Deletes a file or folder and all its children, returning a vec of deleted items
    pub fn delete_entry(&mut self, path: &Utf8Path) -> Vec<(Utf8PathBuf, EditorItem)> {
        let mut deleted = vec![];
        self.fs.retain(|k, v| {
            if k.starts_with(path) {
                deleted.push((k.to_path_buf(), std::mem::take(v)));
                false
            } else {
                true
            }
        });

        self.dirty.extend(deleted.iter().map(|e| e.0.clone()));

        deleted
    }

    pub fn replace_entry(
        &mut self,
        path: &Utf8Path,
        value: EditorItem,
    ) -> anyhow::Result<EditorItem> {
        let file = self.fs.get_mut(path);
        match file {
            None => {
                bail!("File is not found")
            }
            Some(data) => {
                let old = std::mem::replace(data, value);
                self.set_dirty(path.to_path_buf());
                Ok(old)
            }
        }
    }

    pub fn save_to_disk(&mut self) -> Result<(), Vec<Error>> {
        debug!("Saving is starting");
        let mut failures = vec![];
        for (path, data) in self.fs.iter() {
            if !path.starts_with(&self.root) {
                failures.push(anyhow!("File {path} is outside of DBE root directory"));
                continue;
            }
            let is_dirty = self.dirty.remove(path);
            if !is_dirty || data.is_empty() {
                continue;
            }
            debug!("File at {path} is dirty, saving...");

            if let Err(err) = somehow!({
                std::fs::create_dir_all(path.parent().expect("Unexpected root path"))?;
                match data {
                    EditorItem::Raw(data) => std::fs::write(path, data)?,
                    EditorItem::Empty => unreachable!(),
                    EditorItem::Type(..) => bail!("Serialization of Thing files is not supported"),
                    EditorItem::Value(data) => std::fs::write(path, serde_json::to_vec(data)?)?,
                };
            })
            .with_context(|| format!("While saving file at `{path}`"))
            {
                failures.push(err)
            }
        }
        for path in &self.dirty {
            if !path.starts_with(&self.root) {
                failures.push(anyhow!("File {path} is outside of DBE root directory"));
                continue;
            }
            if !path.is_file() {
                failures.push(anyhow::anyhow!(
                    "Entry at `{path}` is not a file (but staged for deletion)"
                ));
            } else if let Err(err) = std::fs::remove_file(path)
                .with_context(|| format!("While deleting file at `{path}`"))
            {
                failures.push(err);
            }
        }
        self.dirty.clear();

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures)
        }
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
        let mut fs: BTreeMap<Utf8PathBuf, EditorItem> = Default::default();
        for (path, value) in self.raw_files {
            fs.insert(path.clone(), EditorItem::Raw(value));
        }
        Ok(DbeFileSystem {
            root: self.root,
            fs,
            dirty: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;
    use rstest::rstest;

    use crate::dbe_files::{DbeFileSystem, EditorItem};

    fn test_fs() -> DbeFileSystem {
        DbeFileSystem {
            root: Utf8PathBuf::from("/user/"),
            fs: Default::default(),
            dirty: Default::default(),
        }
    }

    #[rstest]
    #[case("/")]
    #[case("/some_item.json5")]
    fn create_should_fail_outside(#[case] path: &str) {
        let mut fs = test_fs();
        assert!(fs
            .new_item(EditorItem::Empty, &Utf8PathBuf::from(path))
            .is_err());
    }

    #[rstest]
    #[case("/user/../some_item.json5")]
    fn create_should_fail_breakout(#[case] path: &str) {
        let mut fs = test_fs();
        assert!(fs
            .new_item(EditorItem::Empty, &Utf8PathBuf::from(path))
            .is_err());
    }

    #[rstest]
    #[case("/user/")]
    #[case("/user/.")]
    fn create_should_fail_root(#[case] path: &str) {
        let mut fs = test_fs();
        assert!(fs
            .new_item(EditorItem::Empty, &Utf8PathBuf::from(path))
            .is_err());
    }

    #[rstest]
    #[case("/user/file.json5")]
    #[case("/user/subdir/file.json5")]
    fn create_should_work(#[case] path: &str) {
        let mut fs = test_fs();
        assert!(fs
            .new_item(EditorItem::Empty, &Utf8PathBuf::from(path))
            .is_ok());
    }

    #[rstest]
    #[case("/user/dir")]
    #[case("/user/dir2")]
    #[case("/user/dir3")]
    fn create_should_fail_directory_exists(#[case] path: &str) {
        let mut fs = test_fs();
        for path in [
            "/user/dir/item.json5",
            "/user/dir/item2.json5",
            "/user/dir2/item.json5",
            "/user/dir3/dir4/item.json5",
        ] {
            fs.new_item(EditorItem::Empty, &Utf8PathBuf::from(path))
                .unwrap();
        }
        assert!(fs
            .new_item(EditorItem::Empty, &Utf8PathBuf::from(path))
            .is_err());
        assert!(fs
            .new_item(EditorItem::Empty, &Utf8PathBuf::from(format!("{path}/")))
            .is_err());
        assert!(fs
            .new_item(
                EditorItem::Empty,
                &Utf8PathBuf::from(format!("{path}.json5"))
            )
            .is_ok());
    }
}
