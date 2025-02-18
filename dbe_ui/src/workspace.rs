use crate::error::report_error;
use crate::widgets::collapsible_toolbar::CollapsibleToolbar;
use crate::widgets::dpanel::DPanelSide;
use crate::widgets::report::diagnostic_widget;
use crate::workspace::editors::{editor_for_value, EditorContext};
use crate::workspace::graph::rects::NodeRects;
use crate::workspace::graph::toolbar::{GraphTab, GraphToolbarViewer};
use crate::DbeApp;
use camino::{Utf8Path, Utf8PathBuf};
use dbe_backend::diagnostic::diagnostic::{Diagnostic, DiagnosticLevel};
use dbe_backend::graph::editing::PartialGraphEditingContext;
use dbe_backend::graph::node::SnarlNode;
use dbe_backend::project::docs::DocsRef;
use dbe_backend::project::io::ProjectIO;
use dbe_backend::project::side_effects::SideEffectsContext;
use dbe_backend::project::{
    Project, ProjectFile, EXTENSION_GRAPH, EXTENSION_ITEM, EXTENSION_VALUE,
};
use dbe_backend::validation::validate;
use egui::{Color32, Context, Frame, Margin, RichText, Ui, WidgetText};
use egui_dock::{DockArea, TabViewer};
use egui_hooks::UseHookExt;
use egui_modal::Modal;
use egui_snarl::ui::{SnarlStyle, WireStyle};
use egui_snarl::{NodeId, Snarl};
use egui_toast::{Toast, ToastKind, ToastOptions};
use inline_tweak::tweak;
use itertools::Itertools;
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
        self.show_new_file_modal(ctx, folder, |app, ctx, folder, mut filename| {
            let segments: Vec<&str> = filename.split('.').collect();
            if segments.len() > 1 {
                let ext = segments.last().unwrap().to_lowercase();
                match ext.as_str() {
                    EXTENSION_VALUE | EXTENSION_GRAPH => {
                        app.toasts.push(Toast {
                            kind: ToastKind::Error,
                            text: "Invalid value extension".into(),
                            options: ToastOptions::default()
                                .duration_in_seconds(3.0)
                                .show_progress(true),
                            style: Default::default(),
                        });
                        return;
                    }
                    EXTENSION_ITEM => {
                        filename = filename.to_string();
                    }
                    _ => {
                        filename = format!("{}.{}", filename, EXTENSION_ITEM);
                    }
                }
            } else {
                filename = format!("{}.{}", filename, EXTENSION_ITEM);
            }

            let path = folder.join(filename);
            let Some(project) = app.project.as_mut() else {
                report_error(miette!("No project is open"));
                return;
            };

            if !project.io.is_file_writable(&path).unwrap_or(false) {
                report_error(miette!("Path is not writable"));
                return;
            }

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
        self.show_new_file_modal(ctx, folder, |app, ctx, folder, mut filename| {
            let split: Vec<&str> = filename.split('.').collect();
            if split.len() > 1 {
                let ext = split.last().unwrap().to_lowercase();
                match ext.as_str() {
                    EXTENSION_VALUE | EXTENSION_ITEM => {
                        app.toasts.push(Toast {
                            kind: ToastKind::Error,
                            text: "Invalid graph file extension".into(),
                            options: ToastOptions::default()
                                .duration_in_seconds(3.0)
                                .show_progress(true),
                            style: Default::default(),
                        });
                        return;
                    }
                    EXTENSION_GRAPH => {
                        filename = filename.to_string();
                    }
                    _ => {
                        filename = format!("{}.{}", filename, EXTENSION_GRAPH);
                    }
                }
            } else {
                filename = format!("{}.{}", filename, EXTENSION_GRAPH);
            }

            let path = folder.join(filename);
            let Some(project) = app.project.as_mut() else {
                report_error(miette!("No project is open"));
                return;
            };

            if !project.io.is_file_writable(&path).unwrap_or(false) {
                report_error(miette!("Path is not writable"));
                return;
            }

            if project.files.contains_key(&path) {
                report_error(miette!("File already exists"))
            } else {
                let id = project.graphs.insert_new_graph();
                project.files.insert(path.clone(), ProjectFile::Graph(id));
                app.open_tab_for(ctx, path);
            }
        })
    }

    fn show_new_file_modal(
        &mut self,
        ctx: &Context,
        folder: Utf8PathBuf,
        cb: impl Fn(&mut DbeApp, &Context, &Utf8Path, String) + 'static,
    ) {
        let Some(project) = self.project.as_mut() else {
            report_error(miette!("No project is open"));
            return;
        };
        if !project.io.is_file_writable(&folder).unwrap_or(false) {
            report_error(miette!("Folder is not writable"));
            return;
        }
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
                            let editor = ui.text_edit_singleline(file_name.deref_mut());
                            editor.request_focus();
                        });
                    });
                    let sanitized = sanitize_path(file_name.trim());
                    modal.buttons(ui, |ui| {
                        let can_create = !sanitized.is_empty();
                        ui.add_enabled_ui(can_create, |ui| {
                            if modal.suggested_button(ui, "create").clicked() {
                                let joined = folder.join(sanitized);
                                let folder = joined.parent().unwrap();
                                let name = joined.file_name().unwrap();
                                cb(app, ctx, folder, name.to_string());
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

fn sanitize_path(path: &str) -> String {
    path.split(['/', '\\'])
        .map(|segment| sanitise_file_name::sanitise(segment.trim()))
        .filter(|seg| !seg.is_empty())
        .join("/")
}

pub type Tab = Utf8PathBuf;

struct WorkspaceTabViewer<'a, Io: ProjectIO>(&'a mut Project<Io>);

impl<Io: ProjectIO> TabViewer for WorkspaceTabViewer<'_, Io> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.to_string().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        // self.0
        //     .history
        //     .ensure_file_state(&self.0.files, &self.0.graphs, &tab)
        //     .unwrap_or_else(report_error);

        let Some(data) = self.0.files.get_mut(tab) else {
            ui.centered_and_justified(|ui| {
                ui.label(format!("!!INTERNAL ERROR!! the file {} is missing", tab));
            });
            return;
        };

        let editable = self.0.io.is_file_writable(&tab).unwrap_or(false);

        let mut diagnostics = self.0.diagnostics.enter(tab.as_str());
        let mut changed = false;
        let force_snapshot = false;

        ui.add_enabled_ui(editable, |ui| {
            match data {
                ProjectFile::Value(value) => {
                    let editor = editor_for_value(&self.0.registry, value);

                    // value is always considered changed. Undo history should
                    // figure out when it's actually changed based on hash
                    changed = true;

                    let res = editor.show(
                        ui,
                        EditorContext::new(&self.0.registry, &self.0.docs, DocsRef::None),
                        diagnostics.as_readonly(),
                        "",
                        value,
                    );

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
                ProjectFile::GeneratedValue(value) => {
                    let editor = editor_for_value(&self.0.registry, value);

                    ui.label("Generated value, do not edit");
                    ui.separator();
                    ui.add_enabled_ui(false, |ui| {
                        let res = editor.show(
                            ui,
                            EditorContext::new(&self.0.registry, &self.0.docs, DocsRef::None),
                            diagnostics.as_readonly(),
                            "",
                            value,
                        );
                        assert!(!res.changed, "Generated value was edited")
                    });

                    ui.add_space(ui.ctx().screen_rect().height() * 0.5);
                    ui.separator();
                }
                ProjectFile::BadValue(err) => {
                    let err_str = format!("{:?}", err);

                    ui.label(
                        RichText::new(strip_ansi_escapes::strip_str(err_str)).color(Color32::RED),
                    );
                }
                ProjectFile::Graph(id) => {
                    let Some(graph) = self.0.graphs.graphs.get_mut(id) else {
                        ui.centered_and_justified(|ui| {
                            ui.label(format!("!!INTERNAL ERROR!! the graph {} is missing", id));
                        });
                        return;
                    };

                    // graph is always considered changed. Undo history should
                    // figure out when it's actually changed based on hash
                    changed = true;

                    graph.graph_mut().ensure_region_graph_ready();

                    let is_node_group = graph.is_node_group;

                    let mut selected_nodes = ui.use_state(Vec::<NodeId>::new, ()).into_var();

                    CollapsibleToolbar::new(
                        DPanelSide::Right,
                        &[
                            GraphTab::General,
                            GraphTab::Node,
                            #[cfg(debug_assertions)]
                            GraphTab::Debug,
                        ],
                        &[],
                    )
                    .show_inside(
                        ui,
                        &mut GraphToolbarViewer {
                            graph,
                            selected_nodes: &selected_nodes,
                            registry: &self.0.registry,
                            docs: &self.0.docs,
                        },
                    );

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
                            self.0.graphs.edit_graph(*id, |graph, graphs| {
                                let outputs = &mut None;
                                let (mut ctx, snarl) = PartialGraphEditingContext::from_graph(
                                    graph,
                                    &self.0.registry,
                                    &self.0.docs,
                                    Some(graphs),
                                    SideEffectsContext::unavailable(),
                                    is_node_group,
                                    &[],
                                    outputs,
                                );

                                if let Err(err) = ctx
                                    .as_full(snarl)
                                    .ensure_regions_graph_ready()
                                    .try_as_data()
                                {
                                    diagnostic_widget(
                                        ui,
                                        &Diagnostic {
                                            info: err.into(),
                                            level: DiagnosticLevel::Error,
                                        },
                                    );
                                };

                                let mut rects = ui.use_state(NodeRects::default, ()).into_var();

                                let mut viewer = graph::GraphViewer::new(
                                    ctx,
                                    diagnostics.as_readonly(),
                                    rects.deref_mut(),
                                );

                                let style = SnarlStyle {
                                    wire_style: Some(WireStyle::AxisAligned {
                                        corner_radius: tweak!(32.0),
                                    }),
                                    ..Default::default()
                                };
                                snarl.show(&mut viewer, &style, tab.to_string(), ui);

                                *selected_nodes =
                                    Snarl::<SnarlNode>::get_selected_nodes(tab.to_string(), ui);
                            })
                        });
                }
            }
        });

        drop(diagnostics);
        if changed {
            self.0
                .file_changed(tab, force_snapshot)
                .unwrap_or_else(report_error);
        }
    }
}
