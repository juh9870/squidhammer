use crate::widgets::collapsible_toolbar::ToolbarViewer;
use crate::widgets::rotated_label::RotLabelDirection;
use crate::workspace::graph::toolbar::edit_inputs::edit_inputs_outputs;
use dbe_backend::project::project_graph::ProjectGraph;
use egui::{CollapsingHeader, Ui};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

pub mod edit_inputs;

pub struct GraphToolbarViewer<'a> {
    pub graph: &'a mut ProjectGraph,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GraphTab {
    General,
    Debug,
}

impl ToolbarViewer for GraphToolbarViewer<'_> {
    type Tab = GraphTab;

    fn title(&self, tab: &Self::Tab) -> Cow<'_, str> {
        match tab {
            GraphTab::General => "General".into(),
            GraphTab::Debug => "Debug".into(),
        }
    }

    fn closable(&self, _tab: &Self::Tab) -> bool {
        false
    }

    fn ui(&mut self, ui: &mut Ui, tab: &Self::Tab, _direction: RotLabelDirection) {
        match tab {
            GraphTab::General => {
                let checkbox_res = ui.checkbox(&mut self.graph.is_node_group, "Is Node Group");

                checkbox_res.on_hover_text("When enabled, the graph is treated as node group.\nThis means that other graphs can include this graph into themselves, but this graph will not be executed on its own.");

                ui.add_enabled_ui(self.graph.is_node_group, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut self.graph.name);
                    });

                    edit_inputs_outputs(ui, self.graph.graph_mut());
                });
            }
            GraphTab::Debug => {
                CollapsingHeader::new("Region Graph")
                    .default_open(false)
                    .show(ui, |ui| {
                        let repr = format!("{:#?}", self.graph.graph().region_graph());

                        ui.label(repr);
                    });
                CollapsingHeader::new("Regions data")
                    .default_open(false)
                    .show(ui, |ui| {
                        let repr = format!("{:#?}", self.graph.graph().regions());

                        ui.label(repr);
                    });
            }
        };
    }
}
