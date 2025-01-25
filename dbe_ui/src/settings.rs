use crate::widgets::toggle_button::toggle_button_label;
use egui::{Label, RichText, Ui};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Whether to show a confirmation dialog when the user tries to exit the application.
    #[serde(default = "d_bool::<true>")]
    pub exit_confirmation: bool,
}

impl AppSettings {
    pub fn edit(&mut self, ui: &mut Ui) {
        ui.add_enabled(
            false,
            Label::new(RichText::new("Hover over the labels to see more information").small()),
        );

        toggle_button_label(ui, "Exit Confirmation", &mut self.exit_confirmation)
            .on_hover_text("Show exit confirmation dialog when a project is loaded");
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            exit_confirmation: true,
        }
    }
}

fn d_bool<const VALUE: bool>() -> bool {
    VALUE
}
