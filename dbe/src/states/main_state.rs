use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Context, Error};
use camino::{Utf8Path, Utf8PathBuf};
use derivative::Derivative;
use egui::{menu, Align2, Color32, Id, Pos2, Ui, WidgetText};
use egui_dock::{DockState, Style};
use egui_modal::Modal;
use egui_toast::{Toast, Toasts};
use rust_i18n::t;
use tracing::error;
use undo::History;

use utils::egui::with_temp;
use utils::errors::{display_error, ContextLike};
use utils::{mem_clear, mem_temp, somehow};

use crate::states::main_state::edit::MainStateEdit;
use crate::states::main_state::file_tree::show_file_tree;
use crate::states::main_state::mesh_test::show_mesh_test;
use crate::states::{default_info_panels, DbeFileSystem, DbeStateHolder};
use crate::value::etype::registry::{ETypesRegistry, ETypetId};
use crate::value::EValue;
use crate::vfs::VfsEntry;
use crate::{global_app_scale, scale_ui_style, DbeState};

mod edit;
mod file_tree;
mod mesh_test;

#[derive(Debug)]
struct EditorState {
    fs: DbeFileSystem,
    registry: ETypesRegistry,
    editable_items: BTreeMap<Utf8PathBuf, EValue>,
    dirty: BTreeSet<Utf8PathBuf>,
}

impl EditorState {
    #[inline(always)]
    pub fn set_dirty(&mut self, path: Utf8PathBuf) {
        self.dirty.insert(path);
    }
    pub fn new_item(&mut self, ident: &ETypetId, path: &Utf8Path) -> anyhow::Result<()> {
        somehow!({
            let ty = (self
                .registry
                .get_struct(ident)
                .with_context(|| format!("Failed to get a struct of type `{ident}`")))?;
            // Create a file (mutable operation) as the last source of errors,
            // to avoid doing cleanup
            let entry = (self.fs.fs.create(path))?.path().to_path_buf();
            let value = ty.default_value(&self.registry);
            self.set_dirty(entry.to_path_buf());
            self.editable_items.insert(entry.to_path_buf(), value);
        })
        .with_context(|| format!("While creating file `{path}` of type `{ident}`"))
    }

    pub fn delete_file(&mut self, path: &Utf8Path) -> Result<VfsEntry, Error> {
        somehow!({
            let entry = self.fs.fs.delete(path)?;

            self.dirty
                .extend(entry.iter().map(|e| e.path().to_path_buf()));

            entry
        })
        .with_context(|| format!("While deleting entry at `{path}`"))
    }

    pub fn new_folder(&mut self, path: &Utf8Path) -> anyhow::Result<()> {
        somehow!({
            self.fs.fs.mkdir(path)?;
        })
        .with_context(|| format!("While creating folder at `{path}`"))
    }
}

#[derive(Debug)]
pub struct MainState {
    state: EditorState,
    dock_state: Option<DockState<TabData>>,
    commands_queue: VecDeque<QueuedCommand>,
    edit_history: History<MainStateEdit>,
}

impl MainState {
    pub fn new(fs: DbeFileSystem, registry: ETypesRegistry) -> Self {
        Self {
            state: EditorState {
                fs,
                registry,
                editable_items: Default::default(),
                dirty: Default::default(),
            },
            dock_state: Some(DockState::new(vec![
                TabData::FileTree,
                TabData::MeshTest {
                    indices: vec![],
                    points: vec![],
                },
            ])),
            commands_queue: Default::default(),
            edit_history: Default::default(),
        }
    }

    fn with_reporting<T, CP: ContextLike>(
        &mut self,
        task: impl FnOnce(&mut Self) -> anyhow::Result<T>,
        context: CP,
    ) -> Option<T> {
        match task(self).with_context(|| context.get_context()) {
            Ok(data) => Some(data),
            Err(err) => {
                let err = display_error(err);
                error!(err);
                None
            }
        }
    }

    fn report(&mut self, err: anyhow::Error) {
        let err = display_error(err);
        error!(err);
    }

    pub fn save(&mut self) {}
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
        self.dock_state = Some(state);

        for cmd in commands {
            match cmd {
                TabCommand::CreateNewFile { parent_folder } => self
                    .commands_queue
                    .push_back(QueuedCommand::CreateNewFile { parent_folder }),
                TabCommand::CreateNewFolder { parent_folder } => self
                    .commands_queue
                    .push_back(QueuedCommand::CreateNewFolder { parent_folder }),
                TabCommand::ShowToast(toast) => {
                    toasts.add(toast);
                }
            }
        }

        if let Some(cmd) = self.commands_queue.pop_front() {
            let done = match &cmd {
                QueuedCommand::CreateNewFile { parent_folder } => {
                    create_new_file_modal(ui, &self, parent_folder, &mut edits)
                }
                QueuedCommand::CreateNewFolder { parent_folder } => {
                    create_new_folder_modal(ui, &self, parent_folder, &mut edits)
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

        toasts.show(ui.ctx());

        self.into()
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal_top(|ui| {
            menu::bar(ui, |ui| {
                ui.menu_button(t!("dbe.main.toolbar.file"), |ui| {
                    if ui.button(t!("dbe.main.toolbar.save")).clicked() {
                        self.save();
                        ui.close_menu()
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
}

#[derive(Debug)]
struct TabHandler<'a>(&'a MainState, &'a mut Vec<TabCommand>);

impl<'a> egui_dock::TabViewer for TabHandler<'a> {
    type Tab = TabData;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            TabData::FileTree => t!("dbe.main.file_tree").into(),
            TabData::MeshTest { .. } => t!("dbe.main.mesh_test").into(),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        scale_ui_style(ui);
        match tab {
            TabData::FileTree => show_file_tree(self, ui),
            TabData::MeshTest { indices, points } => show_mesh_test(self, ui, points, indices),
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
    CreateNewFolder { parent_folder: Utf8PathBuf },
    ShowToast(#[derivative(Debug = "ignore")] Toast),
}

#[derive(Clone, Debug)]
enum QueuedCommand {
    CreateNewFile { parent_folder: Utf8PathBuf },
    CreateNewFolder { parent_folder: Utf8PathBuf },
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
                        edits.push(MainStateEdit::CreateFile(
                            ident,
                            parent_folder.join(format!("{name}.json5")),
                        ));
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

fn create_new_folder_modal(
    ui: &mut Ui,
    _page: &MainState,
    parent_folder: &Utf8Path,
    edits: &mut Vec<MainStateEdit>,
) -> bool {
    let id = Id::from("new_folder_modal");
    let modal = Modal::new(ui.ctx(), "new_folder_modal");
    let mut done = false;

    let name_id = id.with("_selected_name");
    modal.show(|ui| {
        scale_ui_style(ui);
        with_temp::<String>(ui, name_id, |ui, name| {
            let mut name = name.unwrap_or_default();
            ui.vertical(|ui| {
                modal.frame(ui, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.label(t!("dbe.main.input_new_folder_name"));
                        ui.text_edit_singleline(&mut name);
                    });
                });

                if modal.button(ui, t!("dbe.generic.ok")).clicked() {
                    edits.push(MainStateEdit::CreateFolder(parent_folder.join(name)));
                    done = true;
                    None
                } else {
                    Some(name)
                }
            })
            .inner
        });
    });
    modal.open();
    if modal.was_outside_clicked() {
        done = true;
    }
    if done {
        mem_clear!(ui, name_id, String)
    }
    done
}
