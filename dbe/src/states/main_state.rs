use crate::states::{DbeFileSystem, DbeStateHolder};
use crate::value::etype::registry::ETypesRegistry;
use crate::vfs::{VfsEntry, VfsEntryType};
use crate::{global_app_scale, scale_style, DbeState};
use camino::{Utf8Path, Utf8PathBuf};
use derivative::Derivative;
use egui::{Align2, Label, Sense, Ui, WidgetText};
use egui_dock::{DockState, Style};
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use rust_i18n::t;

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
            dock_state: Some(DockState::new(vec![TabData::FileTree])),
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
}

struct TabHandler<'a>(&'a mut MainState, &'a mut Vec<TabCommand>);

impl<'a> egui_dock::TabViewer for TabHandler<'a> {
    type Tab = TabData;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            TabData::FileTree => t!("dbe.main.file_tree").into(),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        scale_style(ui.style_mut());
        match tab {
            TabData::FileTree => show_file_tree(self, ui),
        }
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        match tab {
            TabData::FileTree => false,
        }
    }

    fn allowed_in_windows(&self, tab: &mut Self::Tab) -> bool {
        match tab {
            TabData::FileTree => false,
        }
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
enum TabCommand {
    CreateNewFile { parent_folder: Utf8PathBuf },
    CreateNewFolder { parent_folder: Utf8PathBuf },
    ShowToast(#[derivative(Debug = "ignore")] Toast),
}

fn show_file_tree<'a>(state: &'a mut TabHandler, ui: &mut Ui) {
    let selected_path = show_subtree(ui, state.0.fs.fs.root(), state.1);
    if let Some(path) = selected_path {
        state.1.push(TabCommand::ShowToast(Toast {
            kind: ToastKind::Info,
            text: format!("Selected file: {path}").into(),
            options: ToastOptions::default()
                .duration_in_seconds(5.0)
                .show_progress(true),
        }));
    }
}

fn show_subtree<'a>(
    ui: &mut Ui,
    fs: &'a VfsEntry,
    commands: &mut Vec<TabCommand>,
) -> Option<&'a Utf8Path> {
    match fs.ty() {
        VfsEntryType::File(path) => {
            if ui
                .add(Label::new(fs.name()).sense(Sense::click()))
                .double_clicked()
            {
                return Some(path);
            }
            None
        }
        VfsEntryType::Directory(dir) => {
            let response = ui.collapsing(fs.name(), |ui| {
                let mut selected = None;
                for entry in dir.children() {
                    let response = show_subtree(ui, entry, commands);
                    if response.is_some() {
                        selected = response;
                    }
                }
                selected
            });

            response
                .header_response
                .context_menu(|ui| folder_context_menu(ui, fs.path(), commands));

            response.body_returned.flatten()
        }
    }
}

fn folder_context_menu(ui: &mut Ui, path: &Utf8Path, commands: &mut Vec<TabCommand>) {
    if ui.button(t!("dbe.main.new_file")).clicked() {
        commands.push(TabCommand::CreateNewFile {
            parent_folder: path.to_path_buf(),
        });
        ui.close_menu()
    }
    if ui.button(t!("dbe.main.new_folder")).clicked() {
        commands.push(TabCommand::CreateNewFolder {
            parent_folder: path.to_path_buf(),
        });
        ui.close_menu()
    }
}
