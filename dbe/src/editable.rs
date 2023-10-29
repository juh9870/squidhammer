use crate::graph::nodes::{sync_all_nodes_data, sync_node_data, AllEditorNodeTypes};
use crate::graph::{EditorGraphState, EditorState};
use crate::value::etype::registry::ETypesRegistry;
use egui::Ui;
use egui_node_graph::{NodeId, NodeResponse};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EditableFile {
    pub graph: EditorState,
    pub full_screen: Option<NodeId>,
    #[serde(skip)]
    pub synced: bool,
}

impl EditableFile {
    pub fn draw(&mut self, ui: &mut Ui, registry: &Rc<RefCell<ETypesRegistry>>) {
        let mut user_state = EditorGraphState {
            registry: registry.clone(),
        };

        if !self.synced {
            sync_all_nodes_data(&mut self.graph.graph, &mut user_state);
            self.synced = true;
        }

        let res = self.graph.draw_graph_editor(
            ui,
            AllEditorNodeTypes(registry.clone()),
            &mut user_state,
            vec![],
        );

        for res in res.node_responses {
            match res {
                NodeResponse::User(event) => event.apply(self, registry),
                NodeResponse::CreatedNode(id) => {
                    sync_node_data(id, &mut self.graph.graph, &mut user_state)
                }
                NodeResponse::ConnectEventEnded {
                    input,
                    output,
                    input_hook,
                } => {
                    debug!(?input, ?output, input_hook, "Connect")
                }
                _ => {}
            }
        }
    }
}
