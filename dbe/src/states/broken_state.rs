use crate::info_window;
use crate::states::{DbeState, DbeStateHolder};
use egui::Ui;
use egui_file::FileDialog;
use rust_i18n::t;

#[derive(Debug, Default)]
pub struct BrokenState {
    open_folder_dialog: Option<FileDialog>,
}

impl BrokenState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DbeStateHolder for BrokenState {
    fn update(self, ui: &mut Ui) -> DbeState {
        info_window(ui, t!("dbe.broken"), |ui| {
            ui.label(t!("dbe.check_logs"));
        });
        self.into()
    }
}
