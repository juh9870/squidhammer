use std::iter::Peekable;

use camino::Utf8Path;
use egui::{CollapsingHeader, Label, RichText, Sense, Ui};
use itertools::Itertools;
use rust_i18n::t;

use crate::states::main_state::{TabCommand, TabHandler};

pub(super) fn show_file_tree(state: &mut TabHandler, ui: &mut Ui) {
    let reg = state.0.state.registry.borrow();
    let types_root = reg.root_path();
    // let item = state.0.state.fs.fs().iter().peekable()
    show_folder(
        ui,
        state.0.state.fs.root(),
        &mut state.0.state.fs.fs().keys().peekable(),
        &|e| e.starts_with(types_root),
        state.1,
    );
}

fn show_folder(
    ui: &mut Ui,
    path: &Utf8Path,
    fs: &mut Peekable<impl Iterator<Item = impl AsRef<Utf8Path>>>,
    disabled: &impl Fn(&Utf8Path) -> bool,
    commands: &mut Vec<TabCommand>,
) {
    let is_enabled = !disabled(path);
    let mut header = RichText::new(path.file_name().expect("Folder should have a name"));
    if !is_enabled {
        header = header.color(ui.style().visuals.widgets.noninteractive.text_color())
    }
    let response = CollapsingHeader::new(header)
        // .enabled(is_enabled)
        .default_open(is_enabled)
        .show(ui, |ui| {
            while let Some(next) = fs.peek().map(|e| e.as_ref().to_path_buf()) {
                let Ok(remaining) = next.strip_prefix(path) else {
                    break;
                };
                match remaining.components().at_most_one() {
                    Ok(file_name) => {
                        let Some(file_name) = file_name else {
                            panic!("File matches directory name: `{}`", next);
                        };
                        fs.next();
                        if ui
                            .add_enabled(
                                is_enabled,
                                Label::new(file_name.to_string()).sense(Sense::click()),
                            )
                            .double_clicked()
                        {
                            commands.push(TabCommand::OpenFile {
                                path: next.to_path_buf(),
                            });
                        }
                    }
                    Err(mut iter) => {
                        let sub_path = path.join(iter.next().expect("Should not be empty"));
                        show_folder(ui, &sub_path, fs, disabled, commands);
                        // Drain all other items belonging to the sub folder in case they weren't consumed
                        while fs
                            .peek()
                            .map(|e| e.as_ref().starts_with(&sub_path))
                            .unwrap_or(false)
                        {
                            fs.next();
                        }
                    }
                }
            }
        });

    if is_enabled {
        response
            .header_response
            .context_menu(|ui| folder_context_menu(ui, path, commands));
    }
}

fn folder_context_menu(ui: &mut Ui, path: &Utf8Path, commands: &mut Vec<TabCommand>) {
    if ui.button(t!("dbe.main.new_file")).clicked() {
        commands.push(TabCommand::CreateNewFile {
            parent_folder: path.to_path_buf(),
        });
        ui.close_menu()
    }
}
