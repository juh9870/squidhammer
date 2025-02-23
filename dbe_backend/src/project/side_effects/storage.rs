use crate::value::EValue;
use miette::bail;
use std::collections::hash_map::Entry;
use utils::map::HashMap;

#[derive(Debug, Default)]
pub struct TransistentStorage {
    file_scoped: HashMap<EValue, Option<EValue>>,
    global: HashMap<EValue, Option<EValue>>,
    next_stage: HashMap<EValue, (Option<EValue>, String)>,
}

impl TransistentStorage {
    pub fn clear_file_scope(&mut self) {
        self.file_scoped.clear();
    }

    pub fn get(&self, key: &EValue) -> Option<&EValue> {
        self.file_scoped
            .get(key)
            .or_else(|| self.global.get(key))
            .and_then(|x| x.as_ref())
    }

    pub fn insert(&mut self, key: EValue, value: EValue) {
        self.file_scoped.insert(key, Some(value));
    }

    pub fn insert_global(
        &mut self,
        key: EValue,
        value: Option<EValue>,
        emitter: String,
    ) -> miette::Result<()> {
        match self.next_stage.entry(key) {
            Entry::Occupied(entry) => {
                let (_, existing) = entry.get();

                bail!(
                    "Global storage key is modified by multiple nodes. Modified in {}, {}. Key: {}",
                    existing,
                    emitter,
                    entry.key()
                );
            }
            Entry::Vacant(entry) => {
                entry.insert((value, emitter));
                Ok(())
            }
        }
    }

    pub fn flush_stage(&mut self) {
        for (key, (value, _)) in self.next_stage.drain() {
            self.global.insert(key, value);
        }
    }

    /// Unset the value for the given key in the file scope
    ///
    /// Unset key will have no value in the file scope even if it is set in the
    /// global scope
    pub fn unset(&mut self, key: EValue) {
        self.file_scoped.insert(key, None);
    }
}
