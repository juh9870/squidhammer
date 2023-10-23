use crate::graph::EditorState;
use egui_node_graph::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EditableFile {
    pub graph: EditorState,
    pub full_screen: Option<NodeId>,
}
