use crate::states::main_state::{TabCommand, TabHandler};
use crate::vfs::{VfsEntry, VfsEntryType};
use camino::Utf8Path;
use egui::{Label, Sense, Ui};
use egui_toast::{Toast, ToastKind, ToastOptions};
use rust_i18n::t;

pub(super) fn show_file_tree(state: &mut TabHandler, ui: &mut Ui) {
    let selected_path = show_subtree(ui, state.0.fs.fs.root(), state.1);
    if let Some(path) = selected_path {
        state.1.push(TabCommand::ShowToast(Toast {
            kind: ToastKind::Info,
            text: format!("Selected file: {path}").into(),
            options: ToastOptions::default()
                .duration_in_seconds(5.0)
                .show_progress(true),
        }));
    }
}

fn show_subtree<'a>(
    ui: &mut Ui,
    fs: &'a VfsEntry,
    commands: &mut Vec<TabCommand>,
) -> Option<&'a Utf8Path> {
    match fs.ty() {
        VfsEntryType::File(path) => {
            if ui
                .add(Label::new(fs.name()).sense(Sense::click()))
                .double_clicked()
            {
                return Some(path);
            }
            None
        }
        VfsEntryType::Directory(dir) => {
            let response = ui.collapsing(fs.name(), |ui| {
                let mut selected = None;
                for entry in dir.children() {
                    let response = show_subtree(ui, entry, commands);
                    if response.is_some() {
                        selected = response;
                    }
                }
                selected
            });

            response
                .header_response
                .context_menu(|ui| folder_context_menu(ui, fs.path(), commands));

            response.body_returned.flatten()
        }
    }
}

fn folder_context_menu(ui: &mut Ui, path: &Utf8Path, commands: &mut Vec<TabCommand>) {
    if ui.button(t!("dbe.main.new_file")).clicked() {
        commands.push(TabCommand::CreateNewFile {
            parent_folder: path.to_path_buf(),
        });
        ui.close_menu()
    }
    if ui.button(t!("dbe.main.new_folder")).clicked() {
        commands.push(TabCommand::CreateNewFolder {
            parent_folder: path.to_path_buf(),
        });
        ui.close_menu()
    }
}
