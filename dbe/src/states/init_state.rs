use crate::states::{DbeFileSystem, DbeStateHolder};
use crate::{info_window, DbeState};
use egui::Ui;
#[derive(Debug)]
pub struct InitState {
    fs: DbeFileSystem,
}

impl InitState {
    pub fn new(fs: DbeFileSystem) -> Self {
        Self { fs }
    }
}

impl DbeStateHolder for InitState {
    fn update(self, ui: &mut Ui) -> DbeState {
        info_window(ui, "TODO", |_ui| {});
        self.into()
    }
}

impl From<InitState> for DbeState {
    fn from(value: InitState) -> Self {
        DbeState::Initializing(value)
    }
}
