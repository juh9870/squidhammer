use crate::states::loading_state::FilesLoadingState;
use crate::states::DbeStateHolder;
use crate::{info_window, DbeState};
use egui::Ui;
use egui_file::FileDialog;
use rust_i18n::t;
use std::env;

#[derive(Debug, Default)]
pub struct TitleScreenState {
    open_folder_dialog: Option<FileDialog>,
}

impl TitleScreenState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DbeStateHolder for TitleScreenState {
    fn update(mut self, ui: &mut Ui) -> DbeState {
        info_window(ui, t!("dbe.title.open_database"), |ui| {
            ui.vertical_centered_justified(|ui| {
                if ui.button(t!("dbe.title.open_folder")).clicked() {
                    let mut dialog = FileDialog::select_folder(env::current_dir().ok());
                    dialog.open();
                    self.open_folder_dialog = Some(dialog)
                }
            });
            if let Some(dialog) = &mut self.open_folder_dialog {
                let result = dialog.show(ui.ctx());
                if result.selected() {
                    if let Some(p) = dialog.path() {
                        return FilesLoadingState::new(p.to_path_buf()).into();
                    }
                    self.open_folder_dialog = None;
                }
            }
            self.into()
        })
    }
}

impl From<TitleScreenState> for DbeState {
    fn from(value: TitleScreenState) -> Self {
        DbeState::TitleScreen(value)
    }
}
