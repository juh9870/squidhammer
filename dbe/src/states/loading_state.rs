use crate::states::DbeStateHolder;
use crate::DbeState;
use egui::Ui;
use std::path::PathBuf;

pub struct LoadingState {
    path: PathBuf,
}

impl LoadingState {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl DbeStateHolder for LoadingState {
    fn update(self, ui: &mut Ui) -> DbeState {
        self.into()
    }
}

impl From<LoadingState> for DbeState {
    fn from(value: LoadingState) -> Self {
        DbeState::Loading(value)
    }
}
