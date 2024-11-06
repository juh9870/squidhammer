use crate::diagnostics_list::diagnostics_tab;
use crate::error::report_error;
use crate::file_tree::file_tab;
use crate::workspace::Tab;
use ahash::AHashMap;
use color_backtrace::{default_output_stream, BacktracePrinter};
use dbe2::project::Project;
use eframe::epaint::FontFamily;
use eframe::{App, CreationContext, Frame, Storage};
use egui::{Align2, Color32, Context, FontData, FontDefinitions, Ui};
use egui_dock::DockState;
use egui_file::FileDialog;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use egui_tracing::tracing::collector::AllowedTargets;
use egui_tracing::EventCollector;
use itertools::Itertools;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

mod diagnostics_list;
mod error;
mod file_tree;
mod widgets;
mod workspace;

fn main() -> eframe::Result<()> {
    BacktracePrinter::new()
        .add_frame_filter(Box::new(|frame| {
            frame.retain(|frame| {
                if frame
                    .name
                    .as_ref()
                    .is_some_and(|name| name.starts_with("core::ops::function::FnOnce::call_once"))
                {
                    return false;
                }
                true
            })
        }))
        .install(default_output_stream());
    let collector = EventCollector::default()
        .allowed_targets(AllowedTargets::Selected(vec!["dbe".to_string()]));

    let subscriber = tracing_subscriber::Registry::default()
        .with(collector.clone())
        .with(tracing_subscriber::fmt::Layer::default().pretty())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        );

    tracing::subscriber::set_global_default(subscriber).unwrap();

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get().min(16))
        .build_global()
        .unwrap();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "DBE",
        native_options,
        Box::new(|cx| Ok(Box::new(DbeApp::new(cx, collector)))),
    )
}

/// A function that can be called to show a modal
///
/// The function should return true if the modal is done and should no
/// longer be called
type ModalFn = Box<dyn Fn(&mut DbeApp, &Context) -> bool>;

struct DbeApp {
    project: Option<Project>,
    open_file_dialog: Option<FileDialog>,
    collector: EventCollector,
    toasts: Vec<Toast>,
    modals: AHashMap<&'static str, Box<dyn FnMut(&mut DbeApp, &Context) -> bool>>,
    history: Vec<PathBuf>,
    tabs: DockState<Tab>,
}

static ERROR_HAPPENED: AtomicBool = AtomicBool::new(false);

impl DbeApp {
    pub fn new(cx: &CreationContext, collector: EventCollector) -> Self {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "fira-code".to_owned(),
            FontData::from_static(include_bytes!("../fonts/FiraCode-Light.ttf")),
        );

        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "fira-code".to_owned());

        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(1, "fira-code".to_owned());

        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, "fira-code".to_owned());

        cx.egui_ctx.set_fonts(fonts);

        // cx.egui_ctx.style_mut(|s| {
        //     let font_id = FontId {
        //         size: s.text_styles.get(&TextStyle::Body).unwrap().size,
        //         family: FontFamily::Name("fira".into()),
        //     };
        //     s.text_styles.insert(TextStyle::Body, font_id);
        // });

        let mut app = Self {
            project: None,
            open_file_dialog: None,
            collector,
            toasts: vec![],
            modals: Default::default(),
            history: vec![],
            tabs: DockState::new(vec![]),
        };

        if let Some(storage) = cx.storage {
            if let Some(history) = storage.get_string("history") {
                app.history = serde_json5::from_str(&history).unwrap_or_default();
                if let Some(head) = app.history.first() {
                    app.load_project_from_path(head.clone())
                }
            }
        }

        app
    }

    pub fn history_button_list(&mut self, ui: &mut Ui) {
        if self.history.is_empty() {
            ui.colored_label(Color32::GRAY, "No recent projects");
            return;
        }

        let mut want_open: Option<PathBuf> = None;
        for path in &self.history {
            let last = path
                .components()
                .filter(|c| !c.as_os_str().is_empty())
                .last()
                .unwrap();
            if ui.button(last.as_os_str().to_string_lossy()).clicked() {
                want_open = Some(path.clone());
                ui.close_menu();
            }
        }
        if let Some(path) = want_open {
            self.load_project_from_path(path);
        }
    }

    pub fn open_project(&mut self) {
        let mut dialog = FileDialog::select_folder(
            self.project
                .as_ref()
                .map(|p| p.root.as_std_path().to_path_buf()),
        );
        dialog.open();
        self.open_file_dialog = Some(dialog);
    }

    pub fn save_project(&mut self) {
        if let Some(project) = &mut self.project {
            match project.save() {
                Ok(_) => {
                    info!("Project saved successfully");
                    self.toasts.push(Toast {
                        kind: ToastKind::Success,
                        text: "Project saved successfully".into(),
                        options: ToastOptions::default()
                            .duration_in_seconds(3.0)
                            .show_progress(true),
                        style: Default::default(),
                    })
                }
                Err(error) => {
                    report_error(error);
                }
            }
        }
    }

    fn load_project_from_path(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        self.remember_last_project(path.clone());
        match Project::from_path(&path) {
            Ok(data) => {
                self.project = Some(data);
                info!(path=%path.display(), "Project loaded successfully");
                self.toasts.push(Toast {
                    kind: ToastKind::Success,
                    text: "Project loaded successfully".into(),
                    options: ToastOptions::default()
                        .duration_in_seconds(3.0)
                        .show_progress(true),
                    style: Default::default(),
                })
            }
            Err(err) => {
                report_error(err);
            }
        }
    }

    fn remember_last_project(&mut self, path: PathBuf) {
        let index = self.history.iter().find_position(|p| *p == &path);
        if let Some((index, _)) = index {
            self.history.remove(index);
        }

        self.history.insert(0, path);
    }
}

impl App for DbeApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add_enabled(self.project.is_some(), egui::Button::new("Save"))
                        .clicked()
                    {
                        self.save_project();
                        ui.close_menu();
                    }

                    if ui.button("Open").clicked() {
                        self.open_project();
                        ui.close_menu();
                    }

                    ui.menu_button("Recent Projects", |ui| {
                        self.history_button_list(ui);
                    });

                    if ui
                        .add_enabled(self.project.is_some(), egui::Button::new("Close Project"))
                        .clicked()
                    {
                        self.project = None;
                        ui.close_menu();
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close_menu();
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_theme_preference_switch(ui);

                if ui.button("Clear All").clicked() {
                    // self.snarl = Default::default();
                }
            });
        });

        egui::SidePanel::left("files").show(ctx, |ui| file_tab(ui, self));
        egui::SidePanel::right("diagnostics").show(ctx, |ui| diagnostics_tab(ui, self));

        egui::CentralPanel::default().show(ctx, |ui| workspace::workspace(ui, self));

        egui::TopBottomPanel::bottom("bottom_logs")
            .resizable(true)
            .show(ctx, |ui| {
                ui.add(egui_tracing::Logs::new(self.collector.clone()))
            });

        if let Some(dialog) = &mut self.open_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(file) = dialog.path() {
                    let file = file.to_path_buf();
                    self.load_project_from_path(file)
                }
            }
        }

        if ERROR_HAPPENED.swap(false, Ordering::Acquire) {
            self.toasts.push(Toast {
                kind: ToastKind::Error,
                text: "An error has occurred, see console for details".into(),
                options: ToastOptions::default()
                    .duration_in_seconds(3.0)
                    .show_progress(true),
                style: Default::default(),
            });
        }

        let mut toasts = Toasts::new()
            .anchor(Align2::RIGHT_TOP, (-10.0, -10.0))
            .direction(egui::Direction::TopDown);
        for toast in self.toasts.drain(..) {
            toasts.add(toast);
        }
        toasts.show(ctx);

        let mut modals = std::mem::take(&mut self.modals);
        for (_, modal) in &mut modals {
            modal(self, ctx);
        }
        self.modals = modals;
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        if let Ok(history_str) = serde_json5::to_string(&self.history) {
            storage.set_string("history", history_str);
        }
    }
}

/// Helper for wrapping a code block to help with contextualizing errors
/// Better editor support but slightly worse ergonomic than a macro
#[inline(always)]
pub(crate) fn m_try<T>(func: impl FnOnce() -> miette::Result<T>) -> miette::Result<T> {
    func()
}
