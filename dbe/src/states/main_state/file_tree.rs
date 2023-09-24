use camino::Utf8Path;
use egui::{CollapsingHeader, Label, RichText, Sense, Ui, WidgetText};
use egui_toast::{Toast, ToastKind, ToastOptions};
use rust_i18n::t;

use crate::states::main_state::{TabCommand, TabHandler};
use crate::vfs::{VfsEntry, VfsEntryType};

pub(super) fn show_file_tree(state: &mut TabHandler, ui: &mut Ui) {
    let types_root = state.0.state.registry.root_path();
    let selected_path = show_subtree(
        ui,
        state.0.state.fs.fs.root(),
        &|e| e.path().starts_with(types_root),
        state.1,
    );
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
    disabled: &impl Fn(&'a VfsEntry) -> bool,
    commands: &mut Vec<TabCommand>,
) -> Option<&'a Utf8Path> {
    let is_enabled = !disabled(fs);
    match fs.ty() {
        VfsEntryType::File(path) => {
            if ui
                .add_enabled(is_enabled, Label::new(fs.name()).sense(Sense::click()))
                .double_clicked()
            {
                return Some(path);
            }
            None
        }
        VfsEntryType::Directory(dir) => {
            let mut header = RichText::new(fs.name());
            if !is_enabled {
                header = header.color(ui.style().visuals.widgets.noninteractive.text_color())
            }
            let response = CollapsingHeader::new(header)
                // .enabled(is_enabled)
                .show(ui, |ui| {
                    let mut selected = None;
                    for entry in dir.children() {
                        let response = show_subtree(ui, entry, disabled, commands);
                        if response.is_some() {
                            selected = response;
                        }
                    }
                    selected
                });

            if is_enabled {
                response
                    .header_response
                    .context_menu(|ui| folder_context_menu(ui, fs.path(), commands));
            }

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
