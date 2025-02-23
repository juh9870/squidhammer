use crate::m_try;
use crate::project::project_graph::EvaluationStage;
use crate::project::side_effects::mappings::Mappings;
use crate::project::side_effects::storage::TransistentStorage;
use crate::project::{Project, ProjectFile};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use camino::{Utf8Path, Utf8PathBuf};
use egui_snarl::NodeId;
use itertools::Itertools;
use maybe_owned::MaybeOwnedMut;
use miette::{bail, WrapErr};
use std::collections::{btree_map, hash_map, BTreeMap};
use std::hash::{Hash, Hasher};
use tracing::info;
use utils::map::HashMap;
use uuid::Uuid;

pub mod mappings;
pub mod storage;

#[derive(Debug)]
pub enum SideEffect {
    EmitPersistentFile {
        value: EValue,
        path: String,
        is_dbevalue: bool,
    },
    EmitTransientFile {
        value: EValue,
        is_dbevalue: bool,
    },
    SetGlobalStorage {
        key: EValue,
        value: Option<EValue>,
    },
    ShowDebug {
        value: EValue,
    },
}

type SideEffectEmitter = (Utf8PathBuf, Vec<SideEffectPathItem>, usize);

fn clean_up_path(path: &str) -> String {
    path.replace(['\\'], "/")
}

impl SideEffect {
    pub fn execute<Io>(
        self,
        effects: &mut SideEffects,
        project: &mut Project<Io>,
        emitter: SideEffectEmitter,
    ) -> miette::Result<()> {
        fn extension(is_dbevalue: bool) -> &'static str {
            if is_dbevalue {
                ".dbevalue"
            } else {
                ".json"
            }
        }
        fn format_emitter<Io>(emitter: SideEffectEmitter, project: &Project<Io>) -> String {
            format!(
                "{}.{}.{}",
                emitter.0,
                emitter.1[0].to_string(project),
                emitter.2,
            )
        }
        match self {
            SideEffect::EmitPersistentFile {
                value,
                path,
                is_dbevalue,
            } => {
                let path = Utf8PathBuf::from(clean_up_path(&path) + extension(is_dbevalue));
                match project.files.get(&path) {
                    None | Some(ProjectFile::GeneratedValue(..)) => {
                        // ok to overwrite
                    }
                    Some(_) => {
                        bail!("non-generated file already exists at path `{}`", path);
                    }
                }
                project
                    .files
                    .insert(path, ProjectFile::GeneratedValue(value));
            }
            SideEffect::EmitTransientFile { value, is_dbevalue } => {
                let tmp_path = project.registry.project_config().emitted_dir.join(format!(
                    "{}.{}.{}{}",
                    sanitise_file_name::sanitise(emitter.0.as_str()),
                    emitter.1[0].to_string(project),
                    emitter.2,
                    extension(is_dbevalue)
                ));
                project
                    .files
                    .insert(tmp_path, ProjectFile::GeneratedValue(value));
            }
            SideEffect::SetGlobalStorage { key, value } => {
                effects.transistent_storage.insert_global(
                    key,
                    value,
                    format_emitter(emitter, project),
                )?;
            }
            SideEffect::ShowDebug { value } => {
                info!(
                    graph=%emitter.0,
                    path=emitter.1.iter().map(|s|s.to_string(project)).join("->"),
                    index=emitter.2,
                    %value,
                    "Debug",
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SideEffects {
    effects: Vec<(SideEffectEmitter, SideEffect)>,
    mappings: HashMap<Utf8PathBuf, (u64, Mappings)>,
    transistent_storage: TransistentStorage,
    current_stage: EvaluationStage,
}

impl SideEffects {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            mappings: Default::default(),
            transistent_storage: Default::default(),
            current_stage: EvaluationStage::Data,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn push(&mut self, emitter: SideEffectEmitter, effect: SideEffect) {
        self.effects.push((emitter, effect));
    }

    pub fn execute<Io>(
        &mut self,
        project: &mut Project<Io>,
        next_stage: Option<EvaluationStage>,
    ) -> miette::Result<()> {
        let mut iter = 0;
        while !self.effects.is_empty() {
            iter += 1;
            assert!(iter <= 1000, "Side effects formed an infinite loop");
            let mut effects = std::mem::take(&mut self.effects);
            for (emitter, effect) in effects.drain(..) {
                effect.execute(self, project, emitter)?;
            }
            if self.effects.is_empty() {
                self.effects = effects;
            }
        }

        self.save_mappings(project)?;

        if let Some(stage) = next_stage {
            if self.current_stage < stage {
                self.current_stage = stage;
                for (_, m) in self.mappings.values_mut() {
                    m.set_stage(stage);
                }
                self.transistent_storage.flush_stage();
            } else {
                panic!(
                    "Cannot set stage to {:?} from {:?}",
                    stage, self.current_stage
                );
            }
        }

        Ok(())
    }

    pub fn save_mappings<Io>(&mut self, project: &mut Project<Io>) -> miette::Result<()> {
        for (path, (hash, mappings)) in &mut self.mappings {
            m_try(|| {
                match project.files.entry(path.clone()) {
                    btree_map::Entry::Vacant(entry) => {
                        if mappings.has_persistent_ids() {
                            let value = mappings.as_evalue(&project.registry)?;
                            *hash = hash_of(&value);
                            entry.insert(ProjectFile::Value(value));
                        }
                    }
                    btree_map::Entry::Occupied(mut entry) => {
                        let old = entry.get();
                        let ProjectFile::Value(old) = old else {
                            bail!("File `{}` is not a value (persistent)", path);
                        };

                        let file_hash = hash_of(old);

                        if file_hash != *hash {
                            bail!(
                                "Mapping file `{}` has been modified by other side effects",
                                path
                            );
                        }

                        let value = mappings.as_evalue(&project.registry)?;
                        *hash = hash_of(&value);
                        entry.insert(ProjectFile::Value(value));
                    }
                }
                Ok(())
            })
            .with_context(|| format!("failed to save mappings at `{}`", path))?;
        }

        Ok(())
    }

    pub fn load_mappings(
        &mut self,
        registry: &ETypesRegistry,
        files: &BTreeMap<Utf8PathBuf, ProjectFile>,
        path: &Utf8Path,
        ranges: Option<&EValue>,
    ) -> miette::Result<&mut Mappings> {
        m_try(|| match self.mappings.entry(path.to_path_buf()) {
            hash_map::Entry::Occupied(entry) => {
                let mappings = &mut entry.into_mut().1;
                if let Some(ranges) = ranges {
                    mappings.provide_default_ranges(ranges)?;
                }
                Ok(mappings)
            }
            hash_map::Entry::Vacant(entry) => match files.get(path) {
                None => Ok(&mut entry.insert((0, Mappings::new(ranges)?)).1),
                Some(file) => {
                    let ProjectFile::Value(value) = file else {
                        bail!("File `{}` is not a value (persistent)", path);
                    };
                    let hash = hash_of(value);
                    let mappings = Mappings::from_evalue(registry, value)?;
                    Ok(&mut entry.insert((hash, mappings)).1)
                }
            },
        })
        .with_context(|| format!("failed to load mappings at `{}`", path))
    }

    pub fn get_transient_storage(&mut self, key: &EValue) -> Option<&EValue> {
        self.transistent_storage.get(key)
    }

    pub fn set_transient_storage(&mut self, key: EValue, value: EValue) {
        self.transistent_storage.insert(key, value);
    }

    pub fn has_transient_storage(&self, key: &EValue) -> bool {
        self.transistent_storage.get(key).is_some()
    }

    pub fn clear_storage_file_scope(&mut self) {
        self.transistent_storage.clear_file_scope();
    }
}

fn hash_of(value: impl Hash) -> u64 {
    let mut hasher = utils::map::Hasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SideEffectPathItem {
    Node(NodeId),
    Subgraph(Uuid),
}

impl SideEffectPathItem {
    pub fn to_string<IO>(&self, project: &Project<IO>) -> String {
        match self {
            SideEffectPathItem::Node(id) => format!("n{}", id.0),
            SideEffectPathItem::Subgraph(id) => project
                .graphs
                .graphs
                .get(id)
                .map_or_else(|| id.to_string(), |subgraph| subgraph.name.clone()),
        }
    }
}

#[derive(Debug)]
pub enum SideEffectsContext<'a> {
    Context {
        effects: &'a mut SideEffects,
        files: &'a BTreeMap<Utf8PathBuf, ProjectFile>,
        file: Utf8PathBuf,
        path: MaybeOwnedMut<'a, Vec<SideEffectPathItem>>,
        index: MaybeOwnedMut<'a, usize>,
        pop_on_drop: bool,
    },
    Unavailable,
}

impl<'a> SideEffectsContext<'a> {
    pub fn new(
        effect: &'a mut SideEffects,
        file: Utf8PathBuf,
        project_files: &'a BTreeMap<Utf8PathBuf, ProjectFile>,
    ) -> Self {
        Self::Context {
            effects: effect,
            files: project_files,
            file,
            path: MaybeOwnedMut::Owned(Vec::with_capacity(2)),
            index: MaybeOwnedMut::Owned(0),
            pop_on_drop: false,
        }
    }

    pub fn unavailable() -> Self {
        Self::Unavailable
    }

    pub fn is_available(&self) -> bool {
        matches!(self, Self::Context { .. })
    }

    pub fn with_node<'b>(&'b mut self, node: NodeId) -> SideEffectsContext<'b>
    where
        'a: 'b,
    {
        let Self::Context { path, .. } = self else {
            return Self::Unavailable;
        };
        path.push(SideEffectPathItem::Node(node));
        self.clone_inner(true)
    }

    pub fn with_subgraph<'b>(&'b mut self, subgraph: Uuid) -> SideEffectsContext<'b>
    where
        'a: 'b,
    {
        let Self::Context { path, .. } = self else {
            return Self::Unavailable;
        };
        path.push(SideEffectPathItem::Subgraph(subgraph));
        self.clone_inner(true)
    }

    pub fn clone<'b>(&'b mut self) -> SideEffectsContext<'b>
    where
        'a: 'b,
    {
        self.clone_inner(false)
    }

    pub fn clone_inner<'b>(&'b mut self, pop_on_drop: bool) -> SideEffectsContext<'b>
    where
        'a: 'b,
    {
        match self {
            SideEffectsContext::Context {
                effects,
                files,
                file,
                path,
                index,
                pop_on_drop: _,
            } => SideEffectsContext::Context {
                effects,
                files,
                file: file.clone(),
                path: MaybeOwnedMut::Borrowed(path),
                index: MaybeOwnedMut::Borrowed(index),
                pop_on_drop,
            },
            SideEffectsContext::Unavailable => SideEffectsContext::Unavailable,
        }
    }

    pub fn push(&mut self, effect: SideEffect) -> miette::Result<()> {
        match self {
            SideEffectsContext::Context {
                effects,
                file,
                path,
                index,
                ..
            } => {
                let emitter = (file.clone(), path.clone(), **index);
                effects.push(emitter, effect);
                **index += 1;
                Ok(())
            }
            SideEffectsContext::Unavailable => {
                bail!("Side effects context is unavailable");
            }
        }
    }

    pub fn load_mappings(
        &mut self,
        registry: &ETypesRegistry,
        path: &Utf8Path,
        ranges: Option<&EValue>,
    ) -> miette::Result<&mut Mappings> {
        match self {
            SideEffectsContext::Context { effects, files, .. } => {
                effects.load_mappings(registry, files, path, ranges)
            }
            SideEffectsContext::Unavailable => bail!("Side effects context is unavailable"),
        }
    }

    pub fn get_transient_storage(&mut self, key: &EValue) -> miette::Result<Option<&EValue>> {
        match self {
            SideEffectsContext::Context { effects, .. } => Ok(effects.get_transient_storage(key)),
            SideEffectsContext::Unavailable => bail!("Side effects context is unavailable"),
        }
    }

    pub fn set_transient_storage(&mut self, key: EValue, value: EValue) -> miette::Result<()> {
        match self {
            SideEffectsContext::Context { effects, .. } => {
                effects.set_transient_storage(key, value);
                Ok(())
            }
            SideEffectsContext::Unavailable => bail!("Side effects context is unavailable"),
        }
    }

    pub fn has_transient_storage(&self, key: &EValue) -> miette::Result<bool> {
        match self {
            SideEffectsContext::Context { effects, .. } => Ok(effects.has_transient_storage(key)),
            SideEffectsContext::Unavailable => bail!("Side effects context is unavailable"),
        }
    }

    pub fn project_files_iter(
        &self,
    ) -> miette::Result<impl Iterator<Item = (&Utf8PathBuf, &ProjectFile)>> {
        match self {
            SideEffectsContext::Context { files, .. } => Ok(files.iter()),
            SideEffectsContext::Unavailable => bail!("Side effects context is unavailable"),
        }
    }

    // /// Grants edit access to a persistent file in the project.
    // ///
    // /// Will bail if the file is not found or is not a persistent value.
    // pub fn edit_persistent_file_in_place(
    //     &mut self,
    //     path: Utf8PathBuf,
    // ) -> miette::Result<&mut EValue> {
    //     let Self::Context {
    //         files,
    //         effects,
    //         ..
    //     } = self else {
    //         bail!("Side effects context is unavailable");
    //     };
    //
    //     let file = files.get(&path).ok_or_else(|| {
    //         miette!("File `{}` not found in project", path)
    //     })?;
    //
    //     let val = match effects.changed_files.entry(path.clone()) {
    //         Entry::Vacant(e) => {
    //             e.insert(match file {
    //                 ProjectFile::Value(value) => value.clone(),
    //                 ProjectFile::GeneratedValue(_) => bail!("File `{}` is not persistent", path),
    //                 _ => bail!("File `{}` is not a value", path),
    //             })
    //         }
    //         Entry::Occupied(e) => {
    //             e.into_mut()
    //         }
    //     };
    //
    //     Ok(val)
    // }
}

impl Drop for SideEffectsContext<'_> {
    fn drop(&mut self) {
        if let SideEffectsContext::Context {
            path, pop_on_drop, ..
        } = self
        {
            if *pop_on_drop {
                path.pop();
            }
        }
    }
}
