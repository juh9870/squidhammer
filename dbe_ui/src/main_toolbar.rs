use crate::diagnostics_list::diagnostics_tab;
use crate::file_tree::file_tab;
use crate::main_toolbar::colors::colors_tab;
use crate::widgets::collapsible_toolbar::ToolbarViewer;
use crate::widgets::rotated_label::RotLabelDirection;
use crate::DbeApp;
use egui::Ui;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

pub mod colors;
pub mod docs;
pub mod history;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolPanel {
    ProjectTree,
    Diagnostics,
    Log,
    Theme,
    Docs,
    History,
}

pub struct ToolPanelViewer<'a>(pub &'a mut DbeApp);

impl ToolbarViewer for ToolPanelViewer<'_> {
    type Tab = ToolPanel;

    fn title(&self, tab: &Self::Tab) -> Cow<'_, str> {
        match tab {
            ToolPanel::ProjectTree => "Project Tree".into(),
            ToolPanel::Diagnostics => "Diagnostics".into(),
            ToolPanel::Log => "Log".into(),
            ToolPanel::Theme => "Theme".into(),
            ToolPanel::Docs => "Docs".into(),
            ToolPanel::History => "Undo History".into(),
        }
    }

    fn closable(&self, _tab: &Self::Tab) -> bool {
        false
    }

    fn ui(&mut self, ui: &mut Ui, tab: &Self::Tab, _direction: RotLabelDirection) {
        match tab {
            ToolPanel::ProjectTree => file_tab(ui, self.0),
            ToolPanel::Diagnostics => diagnostics_tab(ui, self.0),
            ToolPanel::Log => {
                ui.add(egui_tracing::Logs::new(self.0.collector.clone()));
            }
            ToolPanel::Theme => {
                colors_tab(ui, self.0, true);
            }
            ToolPanel::Docs => {
                docs::docs_tab(ui, self.0);
            }
            ToolPanel::History => {
                history::history_tab(ui, self.0);
            }
        }
    }
}
