#![deny(missing_debug_implementations)]
#![forbid(unsafe_code)]

use camino::Utf8PathBuf;
use egui::{Align2, Id, Ui, WidgetText};
use lazy_static::lazy_static;
use rust_i18n::i18n;

use egui_node_graph::scale::Scale;
pub use states::DbeState;

use crate::graph::{EditorGraph, EditorGraphState};
use crate::states::broken_state::BrokenState;
use crate::states::DbeStateHolder;

mod graph;
mod states;
mod value;
mod vfs;

i18n!();

#[derive(Debug)]
pub struct DbeArguments {
    pub project: Option<Utf8PathBuf>,
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
lazy_static! {
    static ref APP_SCALE_ID: Id = egui::Id::from("APP_SCALE");
}

fn scale_ui_style(ui: &mut Ui) {
    let scale = global_app_scale(ui);
    ui.style_mut().scale(scale);
}

fn global_app_scale(ui: &mut Ui) -> f32 {
    ui.memory_mut(|mem| *mem.data.get_persisted_mut_or(*APP_SCALE_ID, 1f32))
}

pub fn update_dbe(ctx: &egui::Context, data: &mut DbeState) {
    let state = std::mem::replace(data, BrokenState::default().into());
    *data = DbeStateHolder::layout(state, ctx);
}
