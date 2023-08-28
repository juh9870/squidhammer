use crate::commands::Command;
use crate::evaluator::OutputsCache;
use crate::nodes::traits::{IntoNodeInputPort, IntoNodeOutputPort};
use crate::value::etype::MyDataType;
use crate::value::{ENumber, EValue};
use crate::{EditorGraph, MyGraphState};
use egui_node_graph::{Graph, NodeId, NodeTemplateTrait};
use enum_dispatch::enum_dispatch;
use node_macro::editor_node;
use rust_i18n::t;
use smallvec::SmallVec;
use std::borrow::Cow;
use strum_macros::{AsRefStr, EnumIter};

pub mod traits;

#[enum_dispatch]
pub trait EditorNode {
    fn create_ports(
        &self,
        graph: &mut Graph<MyNodeData, MyDataType, EValue>,
        user_state: &mut MyGraphState,
        node_id: NodeId,
    );

    fn categories(&self) -> Vec<&'static str> {
        vec![]
    }

    fn evaluate(
        &self,
        graph: &EditorGraph,
        outputs_cache: &mut OutputsCache,
        commands: &mut Vec<Command>,
        node_id: NodeId,
    ) -> anyhow::Result<()>;
}

/// NodeTemplate is a mechanism to define node templates. It's what the graph
/// will display in the "new node" popup. The user code needs to tell the
/// library how to convert a NodeTemplate into a Node.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, AsRefStr, EnumIter)]
#[enum_dispatch(EditorNode)]
pub enum NodeType {
    Scalar(ScalarMake),
    ScalarAdd(ScalarAdd),
    ScalarSub(ScalarSub),
    ScalarMult(ScalarMult),
    ScalarDiv(ScalarDiv),
    ScalarPrint(ScalarPrint),
}

impl NodeType {
    pub fn label(&self) -> String {
        t!(self.as_ref())
    }
}

/// The NodeData holds a custom data struct inside each node. It's useful to
/// store additional information that doesn't live in parameters. For this
/// example, the node data stores the template (i.e. the "type") of the node.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MyNodeData {
    pub template: NodeType,
}

// A trait for the node kinds, which tells the library how to build new nodes
// from the templates in the node finder
impl NodeTemplateTrait for NodeType {
    type NodeData = MyNodeData;
    type DataType = MyDataType;
    type ValueType = EValue;
    type UserState = MyGraphState;
    type CategoryType = &'static str;

    fn node_finder_label(&self, _user_state: &mut Self::UserState) -> Cow<'_, str> {
        Cow::Owned(self.label())
    }

    // this is what allows the library to show collapsible lists in the node finder.
    fn node_finder_categories(&self, _user_state: &mut Self::UserState) -> Vec<Self::CategoryType> {
        self.categories()
    }

    fn node_graph_label(&self, user_state: &mut Self::UserState) -> String {
        // It's okay to delegate this to node_finder_label if you don't want to
        // show different names in the node finder and the node itself.
        self.node_finder_label(user_state).into()
    }

    fn user_data(&self, _user_state: &mut Self::UserState) -> Self::NodeData {
        MyNodeData { template: *self }
    }

    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
        node_id: NodeId,
    ) {
        self.create_ports(graph, user_state, node_id);
    }
}
pub fn create_input_port<T: IntoNodeInputPort>(
    graph: &mut Graph<MyNodeData, MyDataType, EValue>,
    user_state: &mut MyGraphState,
    node_id: NodeId,
    name: String,
) {
    T::create_input_port(graph, user_state, node_id, name)
}

pub fn create_output_port<T: IntoNodeOutputPort>(
    graph: &mut Graph<MyNodeData, MyDataType, EValue>,
    user_state: &mut MyGraphState,
    node_id: NodeId,
    name: String,
) {
    T::create_output_port(graph, user_state, node_id, name)
}

#[editor_node(name = ScalarMake, outputs = [result])]
pub fn scalar_make(value: ENumber) -> ENumber {
    value
}

#[editor_node(name = ScalarAdd, outputs = [result])]
pub fn scalar_add(values: SmallVec<[ENumber; 2]>) -> ENumber {
    values.iter().sum()
}

#[editor_node(name = ScalarSub, outputs = [result])]
pub fn scalar_sub(a: ENumber, b: ENumber) -> ENumber {
    a - b
}

#[editor_node(name = ScalarMult, outputs = [result])]
pub fn scalar_mult(values: SmallVec<[ENumber; 2]>) -> ENumber {
    values.iter().product()
}

#[editor_node(name = ScalarDiv, outputs = [result])]
pub fn scalar_div(a: ENumber, b: ENumber) -> ENumber {
    a / b
}

#[editor_node(name = ScalarPrint, outputs = [])]
pub fn scalar_print(commands: &mut Vec<Command>, item: ENumber) {
    commands.push(Command::Println(format!("{item}")));
}
