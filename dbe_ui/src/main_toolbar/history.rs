use crate::error::report_error;
use crate::DbeApp;
use dbe_backend::project::undo::{FileSnapshot, SnapshotKind};
use egui::{Button, Ui};
use miette::miette;

pub fn history_tab(ui: &mut Ui, app: &mut DbeApp) {
    ui.label("Undo History");

    let height =
        ui.text_style_height(&egui::TextStyle::Button) + ui.style().spacing.button_padding.y * 2.0;

    let Some(project) = &mut app.project else {
        ui.label("No project loaded");
        return;
    };

    enum Entry<'a> {
        Past(&'a FileSnapshot),
        Present,
        Future(&'a FileSnapshot),
    }

    fn text_for_snapshot(snapshot: &FileSnapshot) -> String {
        match snapshot.kind {
            SnapshotKind::Change => {
                format!("Changed: {} ({})", snapshot.path, snapshot.state)
            }
            SnapshotKind::Undo => {
                format!("Undid change to: {} ({})", snapshot.path, snapshot.state)
            }
        }
    }

    let mut want_undo = false;
    let mut want_redo = false;

    ui.horizontal(|ui| {
        if ui
            .add_enabled(project.history.can_undo(), Button::new("Undo"))
            .clicked()
        {
            want_undo = true;
        }
        if ui
            .add_enabled(project.history.can_redo(), Button::new("Redo"))
            .clicked()
        {
            want_redo = true;
        }
    });

    let history = project.history.history();
    let future = project.history.future();
    let count = history.len() + future.len();
    let items = history.map(Entry::Past).chain(future.map(Entry::Future));

    egui::ScrollArea::vertical().show_rows(ui, height, count, |ui, x| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        for i in items.skip(x.start).take(x.len()) {
            match i {
                Entry::Past(snapshot) => {
                    ui.add_enabled(false, Button::new(text_for_snapshot(snapshot)));
                }
                Entry::Present => {
                    ui.add_enabled(false, Button::new("Current State"));
                }
                Entry::Future(snapshot) => {
                    ui.indent(&snapshot.path, |ui| {
                        ui.add_enabled(false, Button::new(text_for_snapshot(snapshot)));
                    });
                }
            }
        }
    });

    if want_undo && want_redo {
        report_error(miette!("Can't undo and redo at the same time"));
    } else if want_undo {
        app.undo();
    } else if want_redo {
        app.redo();
    }
}
