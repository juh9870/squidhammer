use crate::error::report_error;
use crate::workspace::editors::editor_for_value;
use crate::DbeApp;
use camino::Utf8PathBuf;
use dbe2::graph::execution::partial::PartialGraphExecutionContext;
use dbe2::project::{Project, ProjectFile};
use dbe2::validation::validate;
use egui::{Color32, RichText, Ui, WidgetText};
use egui_dock::{DockArea, TabViewer};
use egui_snarl::ui::SnarlStyle;
use tracing::trace;

pub mod editors;
pub mod graph;

pub fn workspace(ui: &mut Ui, app: &mut DbeApp) {
    if app.project.is_none() {
        ui.centered_and_justified(|ui| ui.label("No project is open"));
        return;
    }
    DockArea::new(&mut app.tabs)
        .style(egui_dock::Style::from_egui(ui.style().as_ref()))
        .show_inside(ui, &mut WorkspaceTabViewer(app.project.as_mut().unwrap()));
}

impl DbeApp {
    pub fn open_tab_for(&mut self, path: Utf8PathBuf) {
        if let Some(tab) = self.tabs.find_tab(&path) {
            self.tabs.set_active_tab(tab)
        } else {
            self.tabs.push_to_focused_leaf(path)
        }
    }
}

pub type Tab = Utf8PathBuf;

struct WorkspaceTabViewer<'a>(&'a mut Project);

impl TabViewer for WorkspaceTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.to_string().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let Some(data) = self.0.files.get_mut(tab) else {
            ui.centered_and_justified(|ui| {
                ui.label(format!("!!INTERNAL ERROR!! the file {} is missing", tab));
            });
            return;
        };

        let mut diagnostics = self.0.diagnostics.enter(tab.as_str());

        match data {
            ProjectFile::Value(value) => {
                let editor = editor_for_value(&self.0.registry, value);

                let res = editor.show(ui, &self.0.registry, diagnostics.as_readonly(), "", value);

                if res.changed {
                    trace!(%tab, "tab value changed, revalidating");
                    if let Err(err) = validate(&self.0.registry, diagnostics, None, value) {
                        report_error(err);
                    }
                }

                ui.label(format!("{:?}", value));
            }
            ProjectFile::BadValue(err) => {
                let err_str = format!("{:?}", err);

                ui.label(RichText::new(strip_ansi_escapes::strip_str(err_str)).color(Color32::RED));
            }
            ProjectFile::Graph(graph) => {
                let (ctx, snarl) =
                    PartialGraphExecutionContext::from_graph(graph, &self.0.registry);

                let mut viewer = graph::GraphViewer {
                    ctx,
                    diagnostics: diagnostics.as_readonly(),
                };

                snarl.show(&mut viewer, &SnarlStyle::default(), tab.to_string(), ui);
            }
        }
    }
}
