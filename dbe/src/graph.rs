use crate::commands::Command;
use crate::evaluator::evaluate_node;
use crate::nodes::data::MyNodeData;
use crate::nodes::EditorNode;
use crate::nodes::NodeType;
use crate::value::etype::MyDataType;
use crate::value::EValue;
use egui_node_graph::{Graph, GraphEditorState, UserResponseTrait};

/// The response type is used to encode side-effects produced when drawing a
/// node in the graph. Most side-effects (creating new nodes, deleting existing
/// nodes, handling connections...) are already handled by the library, but this
/// mechanism allows creating additional side effects from user code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MyResponse {}

/// The graph 'global' state. This state struct is passed around to the node and
/// parameter drawing callbacks. The contents of this struct are entirely up to
/// the user. For this example, we use it to keep track of the 'active' node.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct MyGraphState {}

impl UserResponseTrait for MyResponse {}

pub type EditorGraph = Graph<MyNodeData, MyDataType, EValue>;
pub type MyEditorState = GraphEditorState<MyNodeData, MyDataType, EValue, NodeType, MyGraphState>;

pub fn evaluate_graph(graph: &EditorGraph) -> anyhow::Result<String> {
    let mut cache = Default::default();
    let mut commands = vec![];

    for (id, node) in &graph.nodes {
        if node.user_data.template.has_side_effects() {
            evaluate_node(graph, &mut cache, &mut commands, id)?
        }
    }
    let mut texts = vec![];
    for cmd in commands {
        match cmd {
            Command::Println(line) => {
                texts.push(line);
            }
        }
    }

    Ok(texts.join("\n"))
}
