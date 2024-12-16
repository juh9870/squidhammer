use crate::widgets::collapsible_toolbar::ToolbarViewer;
use crate::widgets::rotated_label::RotLabelDirection;
use egui::Ui;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

pub struct GraphToolbarViewer;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GraphTab {
    General,
}

impl ToolbarViewer for GraphToolbarViewer {
    type Tab = GraphTab;

    fn title(&self, tab: &Self::Tab) -> Cow<'_, str> {
        match tab {
            GraphTab::General => "General".into(),
        }
    }

    fn closable(&self, tab: &Self::Tab) -> bool {
        false
    }

    fn ui(&mut self, ui: &mut Ui, tab: &Self::Tab, direction: RotLabelDirection) {
        match tab {
            GraphTab::General => ui.label("General tab content"),
        };
    }
}
