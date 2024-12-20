use crate::error::report_error;
use crate::widgets::collapsible_toolbar::CollapsibleToolbar;
use crate::widgets::dpanel::DPanelSide;
use crate::workspace::editors::editor_for_value;
use crate::workspace::graph::toolbar::{GraphTab, GraphToolbarViewer};
use crate::DbeApp;
use camino::Utf8PathBuf;
use dbe_backend::graph::editing::PartialGraphEditingContext;
use dbe_backend::project::side_effects::SideEffectsContext;
use dbe_backend::project::{Project, ProjectFile};
use dbe_backend::validation::validate;
use egui::{Color32, Context, Frame, Margin, RichText, Ui, WidgetText};
use egui_dock::{DockArea, TabViewer};
use egui_hooks::UseHookExt;
use egui_modal::Modal;
use egui_snarl::ui::SnarlStyle;
use inline_tweak::tweak;
use miette::miette;
use std::ops::DerefMut;
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
    pub fn open_tab_for(&mut self, _ctx: &Context, path: Utf8PathBuf) {
        if let Some(tab) = self.tabs.find_tab(&path) {
            self.tabs.set_active_tab(tab)
        } else {
            self.tabs.push_to_focused_leaf(path)
        }
    }

    pub fn new_file(&mut self, ctx: &Context, folder: Utf8PathBuf) {
        self.show_new_file_modal(ctx, folder, |app, ctx, folder, filename| {
            let filename = format!("{}.json", filename);
            let path = folder.join(filename);
            let Some(project) = app.project.as_mut() else {
                report_error(miette!("No project is open"));
                return;
            };

            if project.files.contains_key(&path) {
                report_error(miette!("File already exists"))
            } else {
                let value = project
                    .import_root()
                    .default_value(&project.registry)
                    .into_owned();
                project
                    .files
                    .insert(path.clone(), ProjectFile::Value(value));
                app.open_tab_for(ctx, path);
            }
        })
    }

    pub fn new_graph(&mut self, ctx: &Context, folder: Utf8PathBuf) {
        self.show_new_file_modal(ctx, folder, |app, ctx, folder, filename| {
            let filename = format!("{}.dbegraph", filename);
            let path = folder.join(filename);
            let Some(project) = app.project.as_mut() else {
                report_error(miette!("No project is open"));
                return;
            };

            if project.files.contains_key(&path) {
                report_error(miette!("File already exists"))
            } else {
                project
                    .files
                    .insert(path.clone(), ProjectFile::Graph(Default::default()));
                app.open_tab_for(ctx, path);
            }
        })
    }

    fn show_new_file_modal(
        &mut self,
        ctx: &Context,
        folder: Utf8PathBuf,
        cb: impl Fn(&mut DbeApp, &Context, Utf8PathBuf, String) + 'static,
    ) {
        let modal = Modal::new(ctx, "new_file_modal");
        modal.open();
        self.modals.insert(
            "new_file_modal",
            Box::new(move |app, ctx| {
                modal.show(|ui| {
                    let mut file_name = ui.use_state(|| "".to_string(), ()).into_var();
                    modal.title(ui, "New File");
                    modal.frame(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("File name ");
                            ui.text_edit_singleline(file_name.deref_mut());
                        });
                    });
                    let sanitized = sanitise_file_name::sanitise(file_name.trim());
                    modal.buttons(ui, |ui| {
                        let can_create = !sanitized.is_empty();
                        ui.add_enabled_ui(can_create, |ui| {
                            if modal.suggested_button(ui, "create").clicked() {
                                cb(app, ctx, folder.clone(), sanitized);
                            }
                        });
                        if modal.button(ui, "close").clicked() {}
                    });
                });
                modal.is_open()
            }),
        );
    }
}

pub type Tab = Utf8PathBuf;

struct WorkspaceTabViewer<'a, Io>(&'a mut Project<Io>);

impl<Io> TabViewer for WorkspaceTabViewer<'_, Io> {
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

        let mut side_effects = Default::default();
        match data {
            ProjectFile::Value(value) => {
                let editor = editor_for_value(&self.0.registry, value);

                let res = editor.show(ui, &self.0.registry, diagnostics.as_readonly(), "", value);

                if res.changed {
                    trace!(%tab, "tab value changed, revalidating");
                    if let Err(err) =
                        validate(&self.0.registry, diagnostics.enter_inline(), None, value)
                    {
                        report_error(err);
                    }
                }

                ui.add_space(ui.ctx().screen_rect().height() * 0.5);
                ui.separator();
            }
            ProjectFile::BadValue(err) => {
                let err_str = format!("{:?}", err);

                ui.label(RichText::new(strip_ansi_escapes::strip_str(err_str)).color(Color32::RED));
            }
            ProjectFile::Graph(id) => {
                let Some(graph) = self.0.graphs.graphs.get_mut(id) else {
                    ui.centered_and_justified(|ui| {
                        ui.label(format!("!!INTERNAL ERROR!! the graph {} is missing", id));
                    });
                    return;
                };

                CollapsibleToolbar::new(DPanelSide::Right, &[GraphTab::General], &[])
                    .show_inside(ui, &mut GraphToolbarViewer { graph });

                egui::CentralPanel::default()
                    .frame(Frame {
                        inner_margin: Margin {
                            left: tweak!(2.0),
                            right: tweak!(4.0),
                            top: tweak!(1.0),
                            bottom: tweak!(1.0),
                        },
                        ..Default::default()
                    })
                    .show_inside(ui, |ui| {
                        self.0.graphs.edit_graph(*id, |graph, cache, graphs| {
                            let (ctx, snarl) = PartialGraphEditingContext::from_graph(
                                graph,
                                &self.0.registry,
                                Some(graphs),
                                cache,
                                SideEffectsContext::new(&mut side_effects, tab.clone()),
                            );

                            let mut viewer =
                                graph::GraphViewer::new(ctx, diagnostics.as_readonly());

                            snarl.show(&mut viewer, &SnarlStyle::default(), tab.to_string(), ui);
                        })
                    });
            }
            ProjectFile::GeneratedValue(value) => {
                let editor = editor_for_value(&self.0.registry, value);

                ui.label("Generated value, do not edit");
                ui.separator();
                ui.add_enabled_ui(false, |ui| {
                    let res =
                        editor.show(ui, &self.0.registry, diagnostics.as_readonly(), "", value);
                    if res.changed {
                        panic!("Generated value was edited");
                    }
                });

                ui.add_space(ui.ctx().screen_rect().height() * 0.5);
                ui.separator();
            }
        }

        drop(diagnostics);
        side_effects.execute(self.0).unwrap();
    }
}
