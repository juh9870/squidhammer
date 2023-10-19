use std::collections::hash_map::DefaultHasher;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Error};
use camino::{Utf8Path, Utf8PathBuf};
use derivative::Derivative;
use egui::{menu, Align2, Button, Color32, Id, Pos2, Ui, WidgetText};
use egui_dock::{DockState, Node, Style};
use egui_modal::Modal;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use rust_i18n::t;
use tracing::{debug, error, error_span, info, trace, warn};
use undo::History;

use state::EditorData;
use utils::egui::with_temp;
use utils::errors::{display_error, ContextLike};
use utils::{mem_clear, mem_temp};

use crate::dbe_files::{DbeFileSystem, EditorItem};
use crate::states::main_state::edit::MainStateEdit;
use crate::states::main_state::file_edit::show_file_edit;
use crate::states::main_state::file_tree::show_file_tree;
use crate::states::main_state::mesh_test::show_mesh_test;
use crate::states::{default_info_panels, DbeStateHolder};
use crate::value::etype::registry::{ETypesRegistry, ETypetId};
use crate::value::EValue;
use crate::{global_app_scale, scale_ui_style, DbeState};

mod edit;
mod file_edit;
mod file_tree;
mod mesh_test;
mod state;

#[derive(Debug)]
pub struct MainState {
    state: EditorData,
    dock_state: Option<DockState<TabData>>,
    commands_queue: VecDeque<QueuedCommand>,
    edit_history: History<MainStateEdit>,
    last_tab_hash: u64,
    last_snapshot: Instant,
}

const AUTO_COMMIT_DURATION: Duration = Duration::from_secs(5);

impl MainState {
    pub fn new(fs: DbeFileSystem, registry: ETypesRegistry) -> Self {
        Self {
            state: EditorData::new(fs, registry),
            dock_state: Some(DockState::new(vec![
                TabData::FileTree,
                TabData::MeshTest {
                    indices: vec![],
                    points: vec![],
                },
            ])),
            commands_queue: Default::default(),
            edit_history: Default::default(),
            last_tab_hash: 0,
            last_snapshot: Instant::now(),
        }
    }

    #[allow(clippy::needless_arbitrary_self_type)]
    #[duplicate::duplicate_item(
    method                  reference(type);
    [with_reporting]        [& type];
    [with_reporting_mut]    [&mut type];
    )]
    fn method<T, CP: ContextLike>(
        self: reference([Self]),
        task: impl FnOnce(reference([Self])) -> anyhow::Result<T>,
        context: CP,
    ) -> Option<T> {
        match task(self).with_context(|| context.get_context()) {
            Ok(data) => Some(data),
            Err(err) => {
                self.report(err);
                None
            }
        }
    }

    fn report(&self, err: Error) {
        let err = display_error(err);
        error!(err);
    }

    pub fn save_to_disk(&mut self) {
        let span = error_span!("saving");
        let _guard = span.enter();
        self.commit_changes();
        if let Err(errs) = self.state.fs.save_to_disk() {
            if errs.is_empty() {
                info!("Saved completed successfully!")
            } else {
                for x in errs {
                    self.report(x);
                }
                warn!("There were issues during saving, please check log above for errors")
            }
        }
    }

    fn all_tabs(&self) -> Option<impl Iterator<Item = &TabData>> {
        let Some(state) = &self.dock_state else {
            return None;
        };
        Some(
            state
                .iter_nodes()
                .filter_map(|e| {
                    if let Node::Leaf { tabs, .. } = e {
                        Some(tabs)
                    } else {
                        None
                    }
                })
                .flatten(),
        )
    }

    /// Commits changes of all "dirty" file editor windows to the edit history
    pub fn commit_changes(&mut self) {
        trace!(target: "dbe", "Committing changes start");
        let Some(tabs) = self.all_tabs() else {
            return;
        };
        let mut edits = vec![];
        for tab in tabs {
            #[allow(clippy::single_match)]
            match tab {
                TabData::FileEdit { edited_value, path } => {
                    let existing = self.state.fs.content(path);
                    let Some(EditorItem::Value(val)) = existing else {
                        self.report(anyhow!("File at `{path}` is not found, unable to save."));
                        continue;
                    };

                    if val == edited_value {
                        trace!("File at `{path}` is unchanged, skipping");
                        continue;
                    }

                    debug!("Committing changes to {path}");

                    edits.push(MainStateEdit::EditFile {
                        old: None,
                        path: path.clone(),
                        new: edited_value.clone(),
                    });
                }
                _ => {}
            }
        }
        for edit in edits {
            if let Err(err) = self.edit_history.edit(&mut self.state, edit) {
                self.report(err)
            }
        }
    }

    /// Sync content of all edit windows to the current virtual file system
    /// state, discarding all changes since the last commit
    pub fn sync_edit_windows(&mut self) {
        let (Some(tabs), Some(state)) = (self.all_tabs(), &self.dock_state) else {
            return;
        };

        let mut dirty_tabs = vec![];

        for tab in tabs {
            if let TabData::FileEdit { path, edited_value } = tab {
                let existing = self.state.fs.content(path);
                let Some(EditorItem::Value(val)) = existing else {
                    continue; // not our problem
                };
                if edited_value != val {
                    dirty_tabs.push((tab, val.clone()));
                }
            }
        }

        let mut paths = vec![];
        for (tab, data) in dirty_tabs {
            paths.push((state.find_tab(tab).expect("Tab should exist"), data))
        }

        let Some(state) = &mut self.dock_state else {
            panic!("Should have state at this point")
        };

        for ((surface, node, tab), data) in paths {
            let node = &mut state[surface][node];
            match node {
                Node::Leaf { tabs, .. } => {
                    let tab = &mut tabs[tab.0];
                    match tab {
                        TabData::FileEdit { edited_value, .. } => {
                            *edited_value = data;
                        }
                        _ => panic!("Should be a file edit node"),
                    }
                }
                _ => panic!("Should be a leaf node"),
            }
        }
    }
}

impl DbeStateHolder for MainState {
    fn update(mut self, ui: &mut Ui) -> DbeState {
        let mut toasts = Toasts::new()
            .anchor(Align2::RIGHT_TOP, (-10.0, -10.0)) // 10 units from the bottom right corner
            .direction(egui::Direction::TopDown);

        let mut state = std::mem::take(&mut self.dock_state).expect("Docking state is missing");
        let mut style = Style::from_egui(ui.style().as_ref());
        let mut commands = vec![];
        let mut edits = vec![];
        style.tab_bar.height *= global_app_scale(ui);
        egui_dock::DockArea::new(&mut state)
            .style(style)
            .show_inside(ui, &mut TabHandler(&self, &mut commands));

        for cmd in commands {
            match cmd {
                TabCommand::CreateNewFile { parent_folder } => self
                    .commands_queue
                    .push_back(QueuedCommand::CreateNewFile { parent_folder }),
                TabCommand::ShowToast(toast) => {
                    toasts.add(toast);
                }
                TabCommand::OpenFile { path } => {
                    if let Some(EditorItem::Value(value)) = self.state.fs.fs().get(&path) {
                        state.push_to_focused_leaf(TabData::FileEdit {
                            path,
                            edited_value: value.clone(),
                        });
                    } else {
                        toasts.add(Toast {
                            kind: ToastKind::Info,
                            text: format!("File at `{path}` is not editable").into(),
                            options: ToastOptions::default().duration(Duration::from_secs_f64(5.0)),
                        });
                    }
                }
            }
        }

        let cur_tab_hash = state
            .find_active_focused()
            .map(|(_, tab)| {
                let mut h = DefaultHasher::new();
                tab.hash(&mut h);
                h.finish()
            })
            .unwrap_or(0);

        self.dock_state = Some(state);

        if let Some(cmd) = self.commands_queue.pop_front() {
            let done = match &cmd {
                QueuedCommand::CreateNewFile { parent_folder } => {
                    create_new_file_modal(ui, &self, parent_folder, &mut edits)
                }
            };

            if !done {
                self.commands_queue.push_front(cmd);
            }
        }

        for edit in edits {
            let result = self.edit_history.edit(&mut self.state, edit);
            if let Err(err) = result {
                self.edit_history
                    .edit(&mut self.state, MainStateEdit::DeleteLastEdit)
                    .expect("Should delete last edit");
                self.report(err);
            }
        }

        if self.last_tab_hash != cur_tab_hash || self.last_snapshot.elapsed() > AUTO_COMMIT_DURATION
        {
            self.commit_changes();
            self.last_tab_hash = cur_tab_hash;
            self.last_snapshot = Instant::now();
        }

        toasts.show(ui.ctx());

        self.into()
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal_top(|ui| {
            menu::bar(ui, |ui| {
                ui.menu_button(t!("dbe.main.toolbar.file"), |ui| {
                    if ui.button(t!("dbe.main.toolbar.save")).clicked() {
                        self.save_to_disk();
                        ui.close_menu()
                    }
                });
                ui.menu_button(t!("dbe.main.toolbar.edit"), |ui| {
                    if ui
                        .add_enabled(
                            self.edit_history.can_undo(),
                            Button::new(t!("dbe.main.toolbar.undo")),
                        )
                        .clicked()
                    {
                        self.commit_changes();
                        self.edit_history.undo(&mut self.state);
                        self.sync_edit_windows();
                    }
                    if ui
                        .add_enabled(
                            self.edit_history.can_redo(),
                            Button::new(t!("dbe.main.toolbar.redo")),
                        )
                        .clicked()
                    {
                        self.commit_changes();
                        self.edit_history.redo(&mut self.state);
                        self.sync_edit_windows();
                    }
                });
                ui.menu_button(t!("dbe.main.toolbar.misc"), |ui| {
                    default_info_panels(ui);
                })
            });
        });
    }
}

#[derive(Debug)]
pub enum TabData {
    FileTree,
    MeshTest {
        points: Vec<(Pos2, Color32)>,
        indices: Vec<u32>,
    },
    FileEdit {
        path: Utf8PathBuf,
        edited_value: EValue,
    },
}

impl PartialEq for TabData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::FileTree, Self::FileTree) => true,
            // Mesh test is to be removed later, ignore it for now
            (Self::MeshTest { .. }, Self::MeshTest { .. }) => true,
            (Self::FileEdit { path, .. }, Self::FileEdit { path: path2, .. }) => path == path2,
            _ => false,
        }
    }
}

impl Hash for TabData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            TabData::FileTree => {
                0.hash(state);
            }
            TabData::MeshTest { .. } => {
                1.hash(state);
            }
            TabData::FileEdit { path, .. } => {
                2.hash(state);
                path.hash(state);
            }
        }
    }
}

#[derive(Debug)]
struct TabHandler<'a>(&'a MainState, &'a mut Vec<TabCommand>);

impl<'a> egui_dock::TabViewer for TabHandler<'a> {
    type Tab = TabData;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            TabData::FileTree => t!("dbe.main.file_tree").into(),
            TabData::MeshTest { .. } => t!("dbe.main.mesh_test").into(),
            TabData::FileEdit { path, .. } => {
                path.file_name().unwrap_or_else(|| path.as_str()).into()
            }
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        scale_ui_style(ui);
        match tab {
            TabData::FileTree => show_file_tree(self, ui),
            TabData::MeshTest { indices, points } => show_mesh_test(self, ui, points, indices),
            TabData::FileEdit { path, edited_value } => {
                show_file_edit(self, ui, path, edited_value)
            }
        }
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        !matches!(tab, TabData::FileTree | TabData::MeshTest { .. })
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
enum TabCommand {
    CreateNewFile { parent_folder: Utf8PathBuf },
    ShowToast(#[derivative(Debug = "ignore")] Toast),
    OpenFile { path: Utf8PathBuf },
}

#[derive(Clone, Debug)]
enum QueuedCommand {
    CreateNewFile { parent_folder: Utf8PathBuf },
}

fn create_new_file_modal(
    ui: &mut Ui,
    page: &MainState,
    parent_folder: &Utf8Path,
    edits: &mut Vec<MainStateEdit>,
) -> bool {
    let id = Id::new("new_file_modal");
    let modal = Modal::new(ui.ctx(), "new_file_modal");
    let mut done = false;
    let search_id = id.with("_search_text");
    let type_id = id.with("_selected_type");
    let name_id = id.with("_selected_name");
    modal.show(|ui| {
        scale_ui_style(ui);
        match mem_temp!(ui, type_id) {
            Option::<ETypetId>::Some(ident) => with_temp::<String>(ui, name_id, |ui, name| {
                let mut name = name.unwrap_or_default();
                ui.vertical(|ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.label(t!("dbe.main.input_new_item_name"));
                        ui.text_edit_singleline(&mut name);
                    });
                    if modal.button(ui, t!("dbe.generic.ok")).clicked() {
                        if let Some(value) = page.with_reporting(
                            |page| {
                                let ty = page
                                    .state
                                    .registry
                                    .get_struct(&ident)
                                    .context("Unknown struct type")?;
                                Ok(ty.default_value(&page.state.registry))
                            },
                            || format!("While creating struct of type `{ident}`"),
                        ) {
                            edits.push(MainStateEdit::CreateFile(
                                value,
                                parent_folder.join(format!("{name}.json5")),
                            ));
                        }
                        done = true;
                        None
                    } else {
                        Some(name)
                    }
                })
                .inner
            }),
            None => {
                let mut filter_text = mem_temp!(ui, search_id).unwrap_or("".to_string());
                modal.frame(ui, |ui| {
                    ui.label(t!("dbe.generic.search"));
                    let res = ui.text_edit_singleline(&mut filter_text);
                    if res.changed() {
                        mem_temp!(ui, search_id, filter_text.clone());
                    }
                    ui.memory_mut(|mem| mem.request_focus(res.id));
                });
                modal.buttons(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .max_height(ui.available_height())
                        .max_width(ui.available_width())
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                for id in page
                                    .state
                                    .registry
                                    .all_objects()
                                    .filter_map(|e| e.as_struct())
                                    .map(|e| e.ident)
                                    .filter(|e| {
                                        if filter_text.is_empty() {
                                            true
                                        } else {
                                            e.raw().contains(&filter_text)
                                        }
                                    })
                                {
                                    if ui.button(t!(&format!("types.{id}"))).clicked() {
                                        mem_temp!(ui, type_id, id);
                                    }
                                }
                            });
                        });
                });
            }
        }
    });
    modal.open();
    if modal.was_outside_clicked() {
        done = true;
    }

    if done {
        mem_clear!(ui, search_id, String);
        mem_clear!(ui, type_id, ETypetId);
        mem_clear!(ui, name_id, String);
    }

    done
}
