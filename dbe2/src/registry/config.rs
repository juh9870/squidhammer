//! Utilities for reading from the registry's extra configuration

use crate::registry::config::merge::ConfigMerge;
use crate::registry::ETypesRegistry;
use camino::Utf8PathBuf;
use miette::{Context, IntoDiagnostic};
use serde::Deserialize;
use smallvec::SmallVec;
use std::sync::Arc;

pub mod merge;

pub struct ExtraConfig<'a>(pub(super) &'a ETypesRegistry);

impl ExtraConfig<'_> {
    /// Get a value from the extra configuration, caching the result
    pub fn get<T>(&self, key: &str) -> miette::Result<Arc<T>>
    where
        for<'b> T: Deserialize<'b> + ConfigMerge + Default + Send + Sync + 'static,
    {
        let mut cache = self.0.cache.write();
        if !cache.contains_key(key) {
            let value = self.get_uncached::<T>(key)?;

            cache.insert(key.to_string(), Arc::new(value));
        }

        Ok(cache.get(key).unwrap().clone().downcast::<T>().unwrap())
    }

    /// Perform complex actions on the configuration, caching the result
    pub fn cached<
        T: Send + Sync + 'static,
        F: FnOnce(&ExtraConfigsInCache) -> miette::Result<T>,
    >(
        &self,
        cache_key: &str,
        func: F,
    ) -> miette::Result<Arc<T>> {
        let mut cache = self.0.cache.write();
        if !cache.contains_key(cache_key) {
            let value = func(&ExtraConfigsInCache(self))?;

            cache.insert(cache_key.to_string(), Arc::new(value));
        }

        Ok(cache
            .get(cache_key)
            .unwrap()
            .clone()
            .downcast::<T>()
            .unwrap())
    }

    /// Merges all occurrences of a config option into a single value
    pub fn get_uncached<T>(&self, key: &str) -> miette::Result<T>
    where
        for<'b> T: Deserialize<'b> + ConfigMerge + Default,
    {
        let Some(vec) = self.0.extra_config.get(key) else {
            return Ok(Default::default());
        };

        let mut previous = None::<T>;
        let mut paths = SmallVec::<[&Utf8PathBuf; 1]>::new();

        for (path, value) in vec {
            let item = T::deserialize(value).into_diagnostic().with_context(|| {
                format!(
                    "failed to deserialize config option `{}` defined in `{}`",
                    key, path
                )
            })?;
            if let Some(previous) = &mut previous {
                previous.merge(&paths, item, path)?;
            } else {
                previous = Some(item);
            }
            paths.push(path);
        }

        if let Some(previous) = previous {
            Ok(previous)
        } else {
            Ok(Default::default())
        }
    }
}

pub struct ExtraConfigsInCache<'a>(&'a ExtraConfig<'a>);

impl ExtraConfigsInCache<'_> {
    pub fn get<T>(&self, key: &str) -> miette::Result<T>
    where
        for<'b> T: Deserialize<'b> + ConfigMerge + Default,
    {
        self.0.get_uncached(key)
    }
}
