#![deny(missing_debug_implementations)]

use crate::graph::{EditorGraph, EditorGraphState};
use crate::states::loading_state::LoadingState;
use crate::states::title_screen_state::TitleScreenState;
use crate::states::DbeStateHolder;
use egui::{Align2, Ui, Visuals, WidgetText};
use rust_i18n::{i18n, t};
mod graph;
mod states;
mod value;

i18n!();

#[derive(Debug)]
pub enum DbeState {
    Broken,
    TitleScreen(TitleScreenState),
    Loading(LoadingState),
}

impl Default for DbeState {
    fn default() -> Self {
        DbeState::TitleScreen(TitleScreenState::default())
    }
}

fn info_window<T>(
    ui: &mut Ui,
    title: impl Into<WidgetText>,
    content: impl FnOnce(&mut Ui) -> T,
) -> T {
    egui::Window::new(title)
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
            ui.label("TODO: menubar")
        })
    });
    egui::CentralPanel::default().show(ctx, |ui| {
        let state = std::mem::replace(data, DbeState::Broken);
        *data = state.update(ui);
    });
}
