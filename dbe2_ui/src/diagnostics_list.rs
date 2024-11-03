use crate::widgets::report::diagnostics_column;
use crate::DbeApp;
use camino::Utf8PathBuf;
use egui::Ui;

pub fn diagnostics_tab(ui: &mut Ui, app: &mut DbeApp) {
    let Some(project) = &app.project else {
        ui.vertical_centered_justified(|ui| {
            ui.label("No project is open");
        });
        return;
    };

    let mut open_file = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        let diagnostics = &project.diagnostics.diagnostics;

        for (file, errors) in diagnostics {
            if errors.is_empty() {
                continue;
            }
            let state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                ui.id().with(file),
                true,
            );
            state
                .show_header(ui, |ui| {
                    ui.label(file.to_string());
                    if ui.button("Open").clicked() {
                        open_file = Some(Utf8PathBuf::from(file));
                    }
                })
                .body(|ui| {
                    for (path, diagnostic) in errors {
                        if !path.is_empty() {
                            ui.label(path.to_string());
                        }
                        diagnostics_column(ui, diagnostic);
                    }
                });
        }
    });

    if let Some(path) = open_file {
        app.open_tab_for(path);
    }
}
