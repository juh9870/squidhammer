#![deny(missing_debug_implementations)]

use crate::graph::{EditorGraph, EditorGraphState};
use crate::states::init_state::InitState;
use crate::states::loading_state::FilesLoadingState;
use crate::states::title_screen_state::TitleScreenState;
use crate::states::DbeStateHolder;
use bytesize::ByteSize;
use camino::Utf8PathBuf;
use egui::{Align2, Id, Ui, Visuals, WidgetText};
use pluralizer::pluralize;
use rust_i18n::{i18n, t};

mod graph;
mod states;
mod value;
mod vfs;

i18n!();

#[derive(Debug)]
pub struct DbeArguments {
    pub project: Option<Utf8PathBuf>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum DbeState {
    Broken,
    TitleScreen(TitleScreenState),
    Loading(FilesLoadingState),
    Initializing(InitState),
}

impl DbeState {
    pub fn new(args: DbeArguments) -> Self {
        if let Some(path) = args.project {
            Self::Loading(FilesLoadingState::new(path.into()))
        } else {
            Self::TitleScreen(TitleScreenState::default())
        }
    }
}

fn info_window<T>(
    ui: &mut Ui,
    title: impl Into<WidgetText>,
    content: impl FnOnce(&mut Ui) -> T,
) -> T {
    egui::Window::new(title)
        .id(Id::from("info_window"))
        .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .resizable(false)
        .collapsible(false)
        .show(ui.ctx(), |ui| content(ui))
        .expect("Info window is never closed")
        .inner
        .expect("Info window is never collapsed")
}

impl DbeState {
    fn update(self, ui: &mut Ui) -> Self {
        match self {
            DbeState::Broken => {
                info_window(ui, t!("dbe.broken"), |ui| {
                    ui.label(t!("dbe.check_logs"));
                });
                self
            }
            DbeState::TitleScreen(state) => state.update(ui),
            DbeState::Loading(state) => state.update(ui),
            DbeState::Initializing(state) => state.update(ui),
        }
    }
}

pub fn update_dbe(ctx: &egui::Context, data: &mut DbeState) {
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button(t!("dbe.light_mode")).clicked() {
                let visuals = if ui.visuals().dark_mode {
                    Visuals::light()
                } else {
                    Visuals::dark()
                };
                ctx.set_visuals(visuals);
            }
            ui.separator();
            ui.label("TODO: menubar");
            ui.separator();
            ui.label(format!(
                "Ustr cache size: {} ({})",
                ByteSize(ustr::total_allocated() as u64),
                pluralize("entry", ustr::num_entries() as isize, true)
            ));
        })
    });
    egui::CentralPanel::default().show(ctx, |ui| {
        let state = std::mem::replace(data, DbeState::Broken);
        *data = state.update(ui);
    });
}
