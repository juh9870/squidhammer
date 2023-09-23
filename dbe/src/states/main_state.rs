use crate::states::main_state::file_tree::show_file_tree;
use crate::states::main_state::mesh_test::show_mesh_test;
use crate::states::{DbeFileSystem, DbeStateHolder};
use crate::value::etype::registry::ETypesRegistry;
use crate::{global_app_scale, scale_style, DbeState};
use camino::{Utf8Path, Utf8PathBuf};
use derivative::Derivative;
use egui::{Align2, Color32, Pos2, Ui, WidgetText};
use egui_dock::{DockState, Style};
use egui_modal::Modal;
use egui_toast::{Toast, Toasts};
use rust_i18n::t;
use std::collections::VecDeque;
use utils::mem_temp;

mod file_tree;
mod mesh_test;

#[derive(Debug)]
pub struct MainState {
    fs: DbeFileSystem,
    registry: ETypesRegistry,
    dock_state: Option<DockState<TabData>>,
    commands_queue: VecDeque<QueuedCommand>,
}

impl MainState {
    pub fn new(fs: DbeFileSystem, registry: ETypesRegistry) -> Self {
        Self {
            fs,
            registry,
            dock_state: Some(DockState::new(vec![
                TabData::FileTree,
                TabData::MeshTest {
                    indices: vec![],
                    points: vec![],
                },
            ])),
            commands_queue: Default::default(),
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
        style.tab_bar.height *= global_app_scale();
        egui_dock::DockArea::new(&mut state)
            .style(style)
            .show_inside(ui, &mut TabHandler(&mut self, &mut commands));
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
                    create_new_file_modal(ui, &mut self, parent_folder)
                }
                QueuedCommand::CreateNewFolder { .. } => true,
            };

            if !done {
                self.commands_queue.push_front(cmd);
            }
        }

        toasts.show(ui.ctx());

        self.into()
    }
}

impl From<MainState> for DbeState {
    fn from(value: MainState) -> Self {
        DbeState::Main(value)
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

struct TabHandler<'a>(&'a mut MainState, &'a mut Vec<TabCommand>);

impl<'a> egui_dock::TabViewer for TabHandler<'a> {
    type Tab = TabData;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            TabData::FileTree => t!("dbe.main.file_tree").into(),
            TabData::MeshTest { .. } => t!("dbe.main.mesh_test").into(),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        scale_style(ui.style_mut());
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

fn create_new_file_modal(ui: &mut Ui, state: &mut MainState, parent_folder: &Utf8Path) -> bool {
    let modal = Modal::new(ui.ctx(), "new_file_modal");
    let mut done = false;
    let mem_id = ui.id().with("_search_text");
    modal.show(|ui| {
        let mut filter_text = mem_temp!(ui, mem_id).unwrap_or("".to_string());
        modal.frame(ui, |ui| {
            let res = ui.text_edit_singleline(&mut filter_text);
            if res.changed() {
                mem_temp!(ui, mem_id, filter_text.clone());
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
                        for id in state
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
                                modal.close();
                                done = true;
                            }
                        }
                    });
                });
        });
    });
    modal.open();
    if modal.was_outside_clicked() {
        done = true;
    }

    if done {
        mem_temp!(ui, mem_id, "".to_string());
    }

    return done;
}
