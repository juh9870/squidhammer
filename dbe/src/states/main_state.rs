use crate::states::main_state::file_tree::show_file_tree;
use crate::states::main_state::mesh_test::show_mesh_test;
use crate::states::{DbeFileSystem, DbeStateHolder};
use crate::value::etype::registry::ETypesRegistry;
use crate::vfs::{VfsEntry, VfsEntryType};
use crate::{global_app_scale, scale_style, DbeState};
use camino::{Utf8Path, Utf8PathBuf};
use derivative::Derivative;
use egui::epaint::Vertex;
use egui::{
    emath, Align2, Color32, DragValue, Frame, Label, Mesh, Pos2, Rect, Sense, Shape, Slider, Ui,
    WidgetText,
};
use egui_dock::{DockState, Style};
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use itertools::Itertools;
use rust_i18n::t;

mod file_tree;
mod mesh_test;

#[derive(Debug)]
pub struct MainState {
    fs: DbeFileSystem,
    registry: ETypesRegistry,
    dock_state: Option<DockState<TabData>>,
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
                TabCommand::CreateNewFile { .. } => {}
                TabCommand::CreateNewFolder { .. } => {}
                TabCommand::ShowToast(toast) => {
                    toasts.add(toast);
                }
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
