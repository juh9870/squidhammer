use crate::DbeState;
use egui::Ui;

pub mod loading_state;
pub mod title_screen_state;

pub trait DbeStateHolder {
    fn update(self, ui: &mut Ui) -> DbeState;
}
