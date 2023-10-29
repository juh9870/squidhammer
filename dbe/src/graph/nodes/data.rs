use crate::graph::event::EditorGraphResponse;
use crate::graph::nodes::NodeType;
use crate::graph::EditorGraphState;
use crate::value::draw::editor::EFieldEditor;
use crate::value::etype::EDataType;
use crate::value::EValue;
use crate::EditorGraph;
use camino::Utf8PathBuf;
use egui::{Align, Direction, Layout, Ui};
use egui_node_graph::{Graph, NodeDataTrait, NodeId, NodeResponse, UserResponseTrait};
use rustc_hash::FxHashMap;

/// The NodeData holds a custom data struct inside each node. It's useful to
/// store additional information that doesn't live in parameters. For this
/// example, the node data stores the template (i.e. the "type") of the node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditorNodeData {
    pub template: NodeType,
    #[serde(skip)]
    pub editors: FxHashMap<Utf8PathBuf, Box<dyn EFieldEditor>>,
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
        Default::default()
    }

    fn output_ui(
        &self,
        ui: &mut Ui,
        _node_id: NodeId,
        _graph: &Graph<Self, Self::DataType, Self::ValueType>,
        _user_state: &mut Self::UserState,
        param_name: &str,
    ) -> Vec<NodeResponse<Self::Response, Self>>
    where
        Self::Response: UserResponseTrait,
    {
        ui.set_max_width(0.0);
        ui.with_layout(
            Layout::from_main_dir_and_cross_align(Direction::RightToLeft, Align::Min),
            |ui| {
                ui.label(param_name);
            },
        );

        Default::default()
    }
}
