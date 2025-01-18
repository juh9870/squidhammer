use crate::project::project_graph::{ProjectGraph, ProjectGraphs};
use crate::project::ProjectFile;
use crate::value::EValue;
use camino::{Utf8Path, Utf8PathBuf};
use miette::{bail, WrapErr};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use strum::EnumIs;
use tracing::warn;
use utils::map::{hash_of, HashMap};
use utils::ring_stack::RingStack;
use uuid::Uuid;

#[derive(Debug)]
pub struct UndoHistory {
    settings: UndoSettings,
    cur_time: f64,
    change_index: usize,
    last_known_state: HashMap<Utf8PathBuf, u64>,
    last_snapshot: HashMap<Utf8PathBuf, ItemSnapshot>,
    /// Past history
    history: RingStack<FileSnapshot>,
    /// Part of history that was undone. In reverse order.
    undone_history: Vec<FileSnapshot>,
    /// Actions that need to be performed to redo the undone history
    redo_snapshots: Vec<FileSnapshot>,
    flux: Option<Flux>,
}

#[derive(Debug, Clone)]
pub struct UndoSettings {
    /// Maximum number of steps to keep in the undo history.
    pub history_length: usize,
    /// The time in seconds after which a file is considered stable after a change.
    pub stable_time: f64,
    /// If the file is constantly changing, how often to force save a snapshot.
    pub auto_save_interval: f64,
}

impl Default for UndoSettings {
    fn default() -> Self {
        Self {
            history_length: 100,
            stable_time: 1.0,
            auto_save_interval: 10.0,
        }
    }
}

impl UndoHistory {
    pub fn new(settings: UndoSettings) -> Self {
        Self {
            history: RingStack::new(settings.history_length),
            settings,
            cur_time: 0.0,
            change_index: 0,
            last_known_state: Default::default(),
            flux: Default::default(),
            undone_history: Default::default(),
            redo_snapshots: Default::default(),
            last_snapshot: Default::default(),
        }
    }

    pub fn set_time(
        &mut self,
        files: &BTreeMap<Utf8PathBuf, ProjectFile>,
        graphs: &ProjectGraphs,
        time: f64,
    ) {
        self.cur_time = time;
        if let Some(flux) = &self.flux {
            if flux.since_start(self.cur_time) > self.settings.auto_save_interval
                || flux.since_last_change(self.cur_time) > self.settings.stable_time
            {
                self.interrupt_flux(files, graphs).unwrap();
            }
        }
    }

    pub fn ensure_file_state(
        &mut self,
        files: &BTreeMap<Utf8PathBuf, ProjectFile>,
        graphs: &ProjectGraphs,
        path: impl AsRef<Utf8Path>,
    ) -> miette::Result<()> {
        let path = path.as_ref();
        if !self.last_known_state.contains_key(path) {
            self.check_file(files, graphs, path, true)?;
        }

        Ok(())
    }

    /// Notify the undo history that a file may have changed.
    ///
    /// This will check if the file has changed since the last time it was
    /// checked, and if so, save a snapshot of the file.
    ///
    /// A flux system is used to prevent saving too many snapshots of a file,
    /// use [`UndoSettings::stable_time`](field@UndoSettings::stable_time) to
    /// configure how long a file must be stable before a snapshot is saved.
    ///
    /// If [`force_snapshot`] is `true`, a snapshot will be saved regardless of
    /// the flux system.
    pub fn check_file(
        &mut self,
        files: &BTreeMap<Utf8PathBuf, ProjectFile>,
        graphs: &ProjectGraphs,
        path: impl AsRef<Utf8Path>,
        force_snapshot: bool,
    ) -> miette::Result<()> {
        let path = path.as_ref();
        let Some(file) = files.get(path) else {
            bail!("File not found: {:?}", path);
        };

        let state = state_of(file, graphs)?;

        let last_state = if let Some(state) = self.last_known_state.get(path) {
            *state
        } else {
            self.last_known_state.insert(path.to_path_buf(), state);
            state
        };

        if !self.last_snapshot.contains_key(path) {
            let snapshot = ItemSnapshot::from_file(file, graphs)?;
            self.last_snapshot.insert(path.to_path_buf(), snapshot);
        }

        if state == last_state {
            return Ok(());
        }

        self.last_known_state.insert(path.to_path_buf(), state);

        if let Some(flux) = &mut self.flux {
            if force_snapshot || flux.path != *path {
                self.interrupt_flux(files, graphs)?;
            } else {
                // debug!(%path, "File changed, but consumed by flux");
                flux.latest_change_time = self.cur_time;
                return Ok(());
            }
        }

        // debug!(%path, "File changed");

        let new_snapshot = ItemSnapshot::from_file(file, graphs)?;

        debug_assert_eq!(hash_of(&new_snapshot), state);

        let snapshot = self
            .last_snapshot
            .insert(path.to_path_buf(), new_snapshot)
            .expect("Snapshot existence was ensured earlier");

        let snapshot = FileSnapshot {
            id: self.next_change_index(),
            kind: SnapshotKind::Change,
            path: path.to_path_buf(),
            state: hash_of(&snapshot),
            value: snapshot,
        };

        self.push_snapshot(snapshot);

        self.flux = Some(Flux {
            start_time: self.cur_time,
            latest_change_time: self.cur_time,
            path: path.to_path_buf(),
        });

        Ok(())
    }

    /// Undo the last change.
    pub fn undo(
        &mut self,
        files: &mut BTreeMap<Utf8PathBuf, ProjectFile>,
        graphs: &mut ProjectGraphs,
    ) -> miette::Result<Utf8PathBuf> {
        self.interrupt_flux(files, graphs)?;
        let Some(last_snapshot) = self.history.pop() else {
            bail!("Nothing to undo");
        };

        let redo_snapshot = last_snapshot
            .value
            .restore(&last_snapshot.path, files, graphs)?;

        let path = last_snapshot.path.clone();

        self.redo_snapshots.push(FileSnapshot {
            id: last_snapshot.id,
            kind: SnapshotKind::Undo(last_snapshot.id),
            path: path.clone(),
            state: hash_of(&redo_snapshot),
            value: redo_snapshot,
        });

        self.undone_history.push(last_snapshot);

        self.update_last_known_state(path.clone(), files, graphs)
            .context("Failed to recalculate last known file state")?;

        Ok(path)
    }

    /// Redo the last undone change.
    pub fn redo(
        &mut self,
        files: &mut BTreeMap<Utf8PathBuf, ProjectFile>,
        graphs: &mut ProjectGraphs,
    ) -> miette::Result<Utf8PathBuf> {
        self.interrupt_flux(files, graphs)?;
        let Some(redo_snapshot) = self.redo_snapshots.pop() else {
            bail!("Nothing to redo");
        };

        let undone = self
            .undone_history
            .pop()
            .expect("Redo snapshot without undone history");

        debug_assert_eq!(undone.path, redo_snapshot.path);

        let last_snapshot = redo_snapshot
            .value
            .restore(&redo_snapshot.path, files, graphs)?;

        self.history.push(FileSnapshot {
            id: undone.id,
            kind: undone.kind,
            path: redo_snapshot.path.clone(),
            state: hash_of(&last_snapshot),
            value: last_snapshot,
        });

        self.update_last_known_state(redo_snapshot.path.clone(), files, graphs)
            .context("Failed to recalculate last known file state")?;

        Ok(redo_snapshot.path)
    }

    /// Interrupt the flux system, causing the next file change to save a
    /// snapshot regardless of how long ago the file was modified.
    pub fn interrupt_flux(
        &mut self,
        files: &BTreeMap<Utf8PathBuf, ProjectFile>,
        graphs: &ProjectGraphs,
    ) -> miette::Result<()> {
        if let Some(flux) = self.flux.take() {
            let path = flux.path;
            let snapshot = ItemSnapshot::from_file(files.get(&path).unwrap(), graphs)?;
            let last_state = self.last_known_state.get(&path).unwrap();
            if hash_of(&snapshot) != *last_state {
                warn!(%path, "Interrupted flux snapshot differs from last known state, discarding");
            } else {
                // debug!(%path, "Flux interrupted, saving snapshot");
            }
            self.last_snapshot.insert(path.clone(), snapshot);
        }

        Ok(())
    }

    /// Iterator over the change history. From oldest to newest.
    pub fn history(&self) -> impl ExactSizeIterator<Item = &FileSnapshot> {
        self.history.iter()
    }

    /// Iterator over the undone changes. From oldest to newest.
    pub fn undone_history(&self) -> impl ExactSizeIterator<Item = &FileSnapshot> {
        self.undone_history.iter().rev()
    }

    /// Iterator of the undone changes. From oldest to newest.
    pub fn future(&self) -> impl DoubleEndedIterator<Item = &FileSnapshot> + ExactSizeIterator {
        self.redo_snapshots.iter()
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_snapshots.is_empty()
    }
}

impl UndoHistory {
    fn update_last_known_state(
        &mut self,
        path: Utf8PathBuf,
        files: &mut BTreeMap<Utf8PathBuf, ProjectFile>,
        graphs: &mut ProjectGraphs,
    ) -> miette::Result<()> {
        let Some(file) = files.get(&path) else {
            bail!("File not found: {:?}", path);
        };

        let snapshot = ItemSnapshot::from_file(file, graphs)?;

        let state = state_of(file, graphs)?;

        self.last_known_state.insert(path.clone(), state);
        self.last_snapshot.insert(path, snapshot);

        Ok(())
    }

    fn push_snapshot(&mut self, snapshot: FileSnapshot) {
        for x in self.undone_history.drain(..).rev() {
            self.history.push(x);
        }
        for mut x in self.redo_snapshots.drain(..) {
            x.id = self.change_index;
            self.change_index += 1;
            self.history.push(x);
        }
        self.history.push(snapshot);
    }

    fn next_change_index(&mut self) -> usize {
        let index = self.change_index;
        self.change_index += 1;
        index
    }
}

#[derive(Debug)]
struct Flux {
    start_time: f64,
    latest_change_time: f64,
    path: Utf8PathBuf,
}

impl Flux {
    fn since_start(&self, time: f64) -> f64 {
        time - self.start_time
    }

    fn since_last_change(&self, time: f64) -> f64 {
        time - self.latest_change_time
    }
}

#[derive(Debug)]
pub struct FileSnapshot {
    pub id: usize,
    pub kind: SnapshotKind,
    pub path: Utf8PathBuf,
    pub state: u64,
    value: ItemSnapshot,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, EnumIs)]
pub enum SnapshotKind {
    /// A user-made file change
    Change,
    /// An undo hat turned into history snapshot by a new change
    Undo(usize),
}

fn state_of(file: &ProjectFile, graphs: &ProjectGraphs) -> miette::Result<u64> {
    match file {
        ProjectFile::Value(value) => Ok(hash_of(value)),
        ProjectFile::GeneratedValue(_) => {
            bail!("Cannot undo generated values");
        }
        ProjectFile::Graph(id) => Ok(hash_of(&(&id, graphs.graphs.get(id).unwrap()))),
        ProjectFile::BadValue(_) => {
            bail!("Cannot undo bad values");
        }
    }
}

#[derive(Debug, Clone)]
enum ItemSnapshot {
    Value(EValue),
    Graph(Uuid, ProjectGraph),
}

impl Hash for ItemSnapshot {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ItemSnapshot::Value(value) => value.hash(state),
            ItemSnapshot::Graph(id, graph) => {
                (id, graph).hash(state);
            }
        }
    }
}

impl ItemSnapshot {
    fn from_file(file: &ProjectFile, graphs: &ProjectGraphs) -> miette::Result<Self> {
        match file {
            ProjectFile::Value(value) => Ok(Self::Value(value.clone())),
            ProjectFile::GeneratedValue(_) => {
                bail!("Cannot undo generated values");
            }
            ProjectFile::Graph(id) => Ok(Self::Graph(*id, graphs.graphs.get(id).unwrap().clone())),
            ProjectFile::BadValue(_) => {
                bail!("Cannot undo bad values");
            }
        }
    }

    /// Restores the snapshot to the given path, returning the snapshot of the
    /// file that was replaced.
    fn restore(
        &self,
        path: &Utf8PathBuf,
        files: &mut BTreeMap<Utf8PathBuf, ProjectFile>,
        graphs: &mut ProjectGraphs,
    ) -> miette::Result<Self> {
        let mut old_graph = None;
        let value = match &self {
            ItemSnapshot::Value(value) => {
                files.insert(path.clone(), ProjectFile::Value(value.clone()))
            }
            ItemSnapshot::Graph(id, graph) => {
                old_graph = graphs.graphs.insert(*id, graph.clone());
                files.insert(path.clone(), ProjectFile::Graph(*id))
            }
        };

        let value = value.expect("File creation and deletion are handled separately");

        match value {
            ProjectFile::Value(value) => Ok(Self::Value(value)),
            ProjectFile::GeneratedValue(_) => {
                bail!("Cannot undo generated values");
            }
            ProjectFile::Graph(id) => Ok(Self::Graph(id, old_graph.unwrap())),
            ProjectFile::BadValue(_) => {
                bail!("Cannot undo bad values");
            }
        }
    }
}
