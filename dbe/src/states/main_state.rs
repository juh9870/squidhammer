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
use list_edit::list_editor;
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
        match tab {
            TabData::FileTree => false,
            TabData::MeshTest { .. } => false,
            _ => true,
        }
    }

    fn allowed_in_windows(&self, tab: &mut Self::Tab) -> bool {
        match tab {
            _ => true,
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

fn show_file_tree(state: &mut TabHandler, ui: &mut Ui) {
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

fn edit_vec<T>(
    ui: &mut Ui,
    data: &mut Vec<T>,
    new_item: impl FnOnce(usize) -> T,
    item_edit: impl Fn(&mut Ui, usize, &mut T),
) {
    ui.vertical(|ui| {
        enum Command {
            Up(usize),
            Down(usize),
            Delete(usize),
            New,
            None,
        }
        let mut cmd = Command::None;
        let len = data.len();
        for (i, item) in data.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    if ui.add_enabled(i > 0, egui::Button::new("^")).clicked() {
                        cmd = Command::Up(i);
                    }
                    if ui.button("-").clicked() {
                        cmd = Command::Delete(i);
                    }
                    if ui
                        .add_enabled(i != len - 1, egui::Button::new("v"))
                        .clicked()
                    {
                        cmd = Command::Down(i);
                    }
                });
                item_edit(ui, i, item)
            });
        }
        if ui.button("Add").clicked() {
            cmd = Command::New;
        }
        match cmd {
            Command::Up(i) => {
                data.swap(i, i - 1);
            }
            Command::Down(i) => {
                data.swap(i, i + 1);
            }
            Command::Delete(i) => {
                data.remove(i);
            }
            Command::New => {
                data.push(new_item(len));
            }
            Command::None => {}
        }
    });
}

fn show_mesh_test(
    state: &mut TabHandler,
    ui: &mut Ui,
    points: &mut Vec<(Pos2, Color32)>,
    indices: &mut Vec<u32>,
) {
    ui.horizontal_top(|ui| {
        list_editor("vertices")
            .new_item(|_| (Pos2::ZERO, Color32::RED))
            .show(ui, points, |ui, i, data| {
                ui.label(i.index.to_string());
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("x");
                        ui.add(Slider::new(&mut data.0.x, 0.0..=1.0))
                    });
                    ui.horizontal(|ui| {
                        ui.label("y");
                        ui.add(Slider::new(&mut data.0.y, 0.0..=1.0))
                    });
                    let mut color = data.1.to_array();

                    ui.horizontal(|ui| {
                        ui.label("color");
                        ui.color_edit_button_srgba_premultiplied(&mut color);
                    });
                    data.1 =
                        Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]);
                });
            });

        ui.separator();

        list_editor("indices")
            .new_item(|_| 0)
            .show(ui, indices, |ui, _, data| {
                ui.add(DragValue::new(data).clamp_range(0..=points.len()));
            });

        ui.separator();

        Frame::canvas(ui.style()).show(ui, |ui| {
            let (mut response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

            let to_screen = emath::RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
                response.rect,
            );
            let from_screen = to_screen.inverse();

            // let vert = |color: Color32| {
            //     move |x: f32, y: f32| Vertex {
            //         pos: to_screen * Pos2::new(x, y),
            //         uv: Pos2::ZERO,
            //         color,
            //     }
            // };

            // let red = vert(Color32::RED);
            // let green = vert(Color32::GREEN);
            // let blue = vert(Color32::BLUE);

            let vertices = points
                .iter()
                .map(|(p, c)| Vertex {
                    pos: to_screen * *p,
                    color: *c,
                    uv: Pos2::ZERO,
                })
                .collect_vec();
            let mesh = Mesh {
                indices: indices.clone(),
                vertices,
                texture_id: Default::default(),
            };
            painter.extend([Shape::mesh(mesh)]);
            response.mark_changed();

            response
        });
    });
}
