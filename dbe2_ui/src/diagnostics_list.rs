use crate::widgets::report::diagnostics_column;
use crate::DbeApp;
use camino::Utf8PathBuf;
use dbe2::diagnostic::prelude::DiagnosticLevel;
use egui::{Label, RichText, Ui, Widget};
use inline_tweak::tweak;

pub fn diagnostics_tab(ui: &mut Ui, app: &mut DbeApp) {
    let Some(project) = &app.project else {
        ui.vertical_centered_justified(|ui| {
            ui.label("No project is open");
        });
        return;
    };

    let mut open_file = None;

    egui::ScrollArea::both()
        .auto_shrink(tweak!(false))
        .show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            let diagnostics = &project.diagnostics.diagnostics;

            for (file, errors) in diagnostics {
                if errors.is_empty() {
                    continue;
                }
                let state = egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    ui.id().with(file),
                    false,
                );
                state
                    .show_header(ui, |ui| {
                        if ui.button("Open").clicked() {
                            open_file = Some(Utf8PathBuf::from(file));
                        }
                        let max_level = errors
                            .iter()
                            .flat_map(|e| e.1)
                            .map(|d| d.level)
                            .max()
                            .expect("diagnostic list was checked to be non-empty earlier");
                        let count = errors.iter().map(|e| e.1.len()).sum::<usize>();
                        let style = ui.ctx().style();
                        let (color, info) = match max_level {
                            DiagnosticLevel::Error => (style.visuals.error_fg_color, "⚠"),
                            DiagnosticLevel::Warning => (style.visuals.warn_fg_color, "⚠"),
                            _ => (style.visuals.text_color(), "🛈"),
                        };
                        Label::new(RichText::new(format!("{info} {count}")).color(color))
                            .selectable(false)
                            .ui(ui);
                        ui.label(file.to_string());
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
        app.open_tab_for(ui.ctx(), path);
    }
}
