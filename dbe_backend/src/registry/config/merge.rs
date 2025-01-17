use camino::Utf8PathBuf;
use std::hash::{BuildHasher, Hash};

pub trait ConfigMerge {
    fn merge(
        &mut self,
        paths: &[&Utf8PathBuf],
        other: Self,
        other_path: &Utf8PathBuf,
    ) -> miette::Result<()>;
}

impl<T> ConfigMerge for Vec<T> {
    fn merge(
        &mut self,
        _paths: &[&Utf8PathBuf],
        other: Self,
        _other_path: &Utf8PathBuf,
    ) -> miette::Result<()> {
        self.extend(other);

        Ok(())
    }
}

#[allow(clippy::disallowed_types)]
impl<T: Eq + Hash, S: BuildHasher> ConfigMerge for std::collections::HashSet<T, S> {
    fn merge(
        &mut self,
        _paths: &[&Utf8PathBuf],
        other: Self,
        _other_path: &Utf8PathBuf,
    ) -> miette::Result<()> {
        self.extend(other);

        Ok(())
    }
}

#[allow(clippy::disallowed_types)]
impl<K: Eq + Hash, M: ConfigMerge, S: BuildHasher> ConfigMerge
    for std::collections::HashMap<K, M, S>
{
    fn merge(
        &mut self,
        paths: &[&Utf8PathBuf],
        mut other: Self,
        other_path: &Utf8PathBuf,
    ) -> miette::Result<()> {
        for (key, value) in self.iter_mut() {
            if let Some(other) = other.remove(key) {
                value.merge(paths, other, other_path)?;
            }
        }

        // All conflicting keys have been removed from `other`, so we can just extend the map
        self.extend(other);

        Ok(())
    }
}
