use crate::graph::event::EditorGraphResponse;
use crate::graph::nodes::NodeType;
use crate::graph::EditorGraphState;
use crate::value::draw::editor::EFieldEditor;
use crate::value::etype::EDataType;
use crate::value::EValue;
use crate::EditorGraph;
use egui_node_graph::{NodeDataTrait, NodeId, NodeResponse, UserResponseTrait};
use rustc_hash::FxHashMap;

/// The NodeData holds a custom data struct inside each node. It's useful to
/// store additional information that doesn't live in parameters. For this
/// example, the node data stores the template (i.e. the "type") of the node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditorNodeData {
    pub template: NodeType,
    #[serde(skip)]
    pub editors: FxHashMap<String, Box<dyn EFieldEditor>>,
}

impl NodeDataTrait for EditorNodeData {
    type Response = EditorGraphResponse;
    type UserState = EditorGraphState;
    type DataType = EDataType;
    type ValueType = EValue;

    // This method will be called when drawing each node. This allows adding
    // extra ui elements inside the nodes. In this case, we create an "active"
    // button which introduces the concept of having an active node in the
    // graph. This is done entirely from user code with no modifications to the
    // node graph library.
    fn bottom_ui(
        &self,
        _ui: &mut egui::Ui,
        _node_id: NodeId,
        _graph: &EditorGraph,
        _user_state: &mut Self::UserState,
    ) -> Vec<NodeResponse<EditorGraphResponse, EditorNodeData>>
    where
        EditorGraphResponse: UserResponseTrait,
    {
        vec![]
    }
}
