use crate::project::{Project, ProjectFile};
use crate::value::EValue;
use camino::Utf8PathBuf;
use egui_snarl::NodeId;
use maybe_owned::MaybeOwnedMut;
use miette::bail;
use uuid::Uuid;

#[derive(Debug)]
pub enum SideEffect {
    EmitPersistentFile { value: EValue, path: Utf8PathBuf },
    EmitTransientFile { value: EValue },
}

type SideEffectEmitter = (Utf8PathBuf, Vec<SideEffectPathItem>, usize);

impl SideEffect {
    pub fn execute<Io>(
        self,
        emitter: SideEffectEmitter,
        project: &mut Project<Io>,
    ) -> miette::Result<()> {
        match self {
            SideEffect::EmitPersistentFile { value, path } => {
                match project.files.get(&path) {
                    None => {}
                    Some(ProjectFile::GeneratedValue(_)) => {
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
            SideEffect::EmitTransientFile { value } => {
                let tmp_path = project.registry.project_config().emitted_dir.join(format!(
                    "{}.n{}.{}.json",
                    sanitise_file_name::sanitise(emitter.0.as_str()),
                    emitter.1[0].to_string(project),
                    emitter.2
                ));
                project
                    .files
                    .insert(tmp_path, ProjectFile::GeneratedValue(value));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SideEffects {
    effects: Vec<(SideEffectEmitter, SideEffect)>,
}

impl SideEffects {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn push(&mut self, emitter: SideEffectEmitter, effect: SideEffect) {
        self.effects.push((emitter, effect));
    }

    pub fn execute<Io>(&mut self, project: &mut Project<Io>) -> miette::Result<()> {
        let mut iter = 0;
        while !self.effects.is_empty() {
            iter += 1;
            if iter > 1000 {
                panic!("Side effects formed an infinite loop");
            }
            let mut effects = std::mem::take(&mut self.effects);
            for (emitter, effect) in effects.drain(..) {
                effect.execute(emitter, project)?;
            }
            if self.effects.is_empty() {
                self.effects = effects;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SideEffectPathItem {
    Node(NodeId),
    Subgraph(Uuid),
}

impl SideEffectPathItem {
    pub fn to_string<IO>(&self, project: &Project<IO>) -> String {
        match self {
            SideEffectPathItem::Node(id) => id.0.to_string(),
            SideEffectPathItem::Subgraph(id) => project
                .graphs
                .graphs
                .get(id)
                .map(|subgraph| subgraph.name.clone())
                .unwrap_or_else(|| id.to_string()),
        }
    }
}

#[derive(Debug)]
pub struct SideEffectsContext<'a> {
    effect: &'a mut SideEffects,
    file: Utf8PathBuf,
    path: MaybeOwnedMut<'a, Vec<SideEffectPathItem>>,
    index: usize,
    pop_on_drop: bool,
}

impl<'a> SideEffectsContext<'a> {
    pub fn new(effect: &'a mut SideEffects, file: Utf8PathBuf) -> Self {
        Self {
            effect,
            file,
            path: MaybeOwnedMut::Owned(Vec::with_capacity(2)),
            index: 0,
            pop_on_drop: false,
        }
    }

    pub fn with_node<'b>(&'b mut self, node: NodeId) -> SideEffectsContext<'b>
    where
        'a: 'b,
    {
        self.path.push(SideEffectPathItem::Node(node));
        SideEffectsContext {
            effect: self.effect,
            file: self.file.clone(),
            path: MaybeOwnedMut::Borrowed(&mut self.path),
            index: 0,
            pop_on_drop: true,
        }
    }

    pub fn with_subgraph<'b>(&'b mut self, subgraph: Uuid) -> SideEffectsContext<'b>
    where
        'a: 'b,
    {
        self.path.push(SideEffectPathItem::Subgraph(subgraph));
        SideEffectsContext {
            effect: self.effect,
            file: self.file.clone(),
            path: MaybeOwnedMut::Borrowed(&mut self.path),
            index: 0,
            pop_on_drop: true,
        }
    }

    pub fn clone<'b>(&'b mut self) -> SideEffectsContext<'b>
    where
        'a: 'b,
    {
        SideEffectsContext {
            effect: self.effect,
            file: self.file.clone(),
            path: MaybeOwnedMut::Borrowed(&mut self.path),
            index: 0,
            pop_on_drop: false,
        }
    }

    pub fn push(&mut self, effect: SideEffect) {
        let emitter = (self.file.clone(), self.path.clone(), self.index);
        self.effect.push(emitter, effect);
        self.index += 1;
    }
}

impl Drop for SideEffectsContext<'_> {
    fn drop(&mut self) {
        if self.pop_on_drop {
            self.path.pop();
        }
    }
}
