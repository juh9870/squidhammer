use crate::DbeApp;
use camino::{Utf8Path, Utf8PathBuf};
use dbe_backend::project::io::ProjectIO;
use dbe_backend::project::Project;
use egui::{CollapsingHeader, Context, Label, RichText, Sense, Ui};
use inline_tweak::tweak;
use itertools::Itertools;
use std::iter::Peekable;

#[derive(Debug)]
enum Command {
    OpenFile { path: Utf8PathBuf },
    NewFile { folder: Utf8PathBuf },
    NewGraph { folder: Utf8PathBuf },
}

pub fn file_tab(ui: &mut Ui, app: &mut DbeApp) {
    // dbg!(("file tab",ui.available_size()));
    egui::ScrollArea::both()
        .auto_shrink(tweak!(false))
        .show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            if let Some(project) = &mut app.project {
                let commands = file_tree(ui, project);
                consume_commands(commands, app, ui.ctx());
            } else {
                if ui.button("Open Project").clicked() {
                    app.open_project();
                }

                ui.group(|ui| {
                    ui.label("Recent projects");
                    app.history_button_list(ui);
                });
            }
        });
}

fn file_tree<Io: ProjectIO>(ui: &mut Ui, project: &mut Project<Io>) -> Vec<Command> {
    let mut commands = vec![];

    let io = &project.io;
    show_folder(
        ui,
        "".as_ref(),
        &mut project.files.keys().peekable(),
        &|_| false,
        &|path| io.is_file_writable(path).unwrap_or(false),
        &mut commands,
    );

    commands
}

fn consume_commands(commands: Vec<Command>, app: &mut DbeApp, ctx: &Context) {
    for cmd in commands {
        match cmd {
            Command::OpenFile { path } => app.open_tab_for(ctx, path),
            Command::NewFile { folder } => app.new_file(ctx, folder),
            Command::NewGraph { folder } => app.new_graph(ctx, folder),
        }
    }
}

fn show_folder(
    ui: &mut Ui,
    path: &Utf8Path,
    fs: &mut Peekable<impl Iterator<Item = impl AsRef<Utf8Path>>>,
    disabled: &impl Fn(&Utf8Path) -> bool,
    editable: &impl Fn(&Utf8Path) -> bool,
    commands: &mut Vec<Command>,
) {
    let is_enabled = !disabled(path);
    let is_editable = editable(path);

    ui.push_id(path, |ui| {
        if !is_editable {
            let non_editable_color = ui
                .style()
                .visuals
                .widgets
                .noninteractive
                .text_color()
                .gamma_multiply(tweak!(0.5));

            if ui.style().visuals.widgets.inactive.fg_stroke.color != non_editable_color {
                ui.style_mut().visuals.widgets.inactive.fg_stroke.color = non_editable_color;
                ui.style_mut().visuals.widgets.hovered.fg_stroke.color = ui
                    .style_mut()
                    .visuals
                    .widgets
                    .hovered
                    .fg_stroke
                    .color
                    .gamma_multiply(tweak!(0.66));
            }
        }

        let header = RichText::new(path.file_name().unwrap_or("Project Root"));
        let response = CollapsingHeader::new(header)
            // .enabled(is_enabled)
            .default_open(is_enabled)
            .show(ui, |ui| {
                let mut files = vec![];
                let mut folders = vec![];
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
                            let name = file_name.to_string();
                            files.push((next, name));
                        }
                        Err(mut iter) => {
                            let sub_path = path.join(iter.next().expect("Should not be empty"));
                            let mut folder_items = vec![];
                            while fs.peek().is_some_and(|e| e.as_ref().starts_with(&sub_path)) {
                                folder_items
                                    .push(fs.next().expect("Peeked item should be present"));
                            }
                            folders.push((sub_path, folder_items));
                        }
                    }
                }

                for (sub_path, folder) in folders {
                    show_folder(
                        ui,
                        &sub_path,
                        &mut folder.into_iter().peekable(),
                        disabled,
                        editable,
                        commands,
                    );
                }
                for (file, file_name) in files {
                    let text = RichText::new(file_name);
                    if ui
                        .add_enabled(
                            is_enabled,
                            Label::new(text).sense(Sense::click()).selectable(false),
                        )
                        .double_clicked()
                    {
                        commands.push(Command::OpenFile {
                            path: file.to_path_buf(),
                        });
                    }
                }
            });

        if is_enabled && is_editable {
            response
                .header_response
                .context_menu(|ui| folder_context_menu(ui, path, commands));
        }
    });
}

fn folder_context_menu(ui: &mut Ui, path: &Utf8Path, commands: &mut Vec<Command>) {
    ui.menu_button("Create", |ui| {
        if ui.button("File").clicked() {
            commands.push(Command::NewFile {
                folder: path.to_path_buf(),
            });
            ui.close_menu();
        }
        if ui.button("Graph").clicked() {
            commands.push(Command::NewGraph {
                folder: path.to_path_buf(),
            });
            ui.close_menu();
        }
    });
}
