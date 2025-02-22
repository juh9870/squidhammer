use crate::widgets::toggle_button::toggle_button_label;
use egui::{DragValue, Label, RichText, Ui, Widget};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "d_bool::<true>")]
    pub exit_confirmation: bool,
    #[serde(default = "d_bool::<false>")]
    pub autosave: bool,
    #[serde(default = "d_u32::<60>")]
    pub autosave_interval: u32,
    #[serde(default = "d_bool::<true>")]
    pub check_for_updates: bool,
}

impl AppSettings {
    pub fn edit(&mut self, ui: &mut Ui) {
        ui.add_enabled(
            false,
            Label::new(RichText::new("Hover over the labels to see more information").small()),
        );

        toggle_button_label(ui, &mut self.check_for_updates, "Check for updates")
            .on_hover_text("Check for updates on startup");

        toggle_button_label(ui, &mut self.exit_confirmation, "Exit Confirmation")
            .on_hover_text("Show exit confirmation dialog when a project is loaded");

        toggle_button_label(ui, &mut self.autosave, "Autosave")
            .on_hover_text("Automatically save the project at a set interval");

        ui.add_enabled_ui(self.autosave, |ui| {
            ui.indent("autosave_fields", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Autosave Interval (s)")
                        | DragValue::new(&mut self.autosave_interval)
                            .speed(1.0)
                            .range(30..=3600)
                            .ui(ui)
                })
                .inner
                .on_hover_text(
                    "The interval in seconds at which the project is automatically saved",
                );
            });
        });
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            exit_confirmation: true,
            autosave: false,
            autosave_interval: 60,
            check_for_updates: true,
        }
    }
}

fn d_bool<const VALUE: bool>() -> bool {
    VALUE
}

fn d_u32<const VALUE: u32>() -> u32 {
    VALUE
}
