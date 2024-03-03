use crate::graph::commands::Command;
use crate::graph::evaluator::OutputsCache;
use crate::graph::nodes::data::EditorNodeData;
use crate::graph::nodes::scalar::{
    ScalarAdd, ScalarDiv, ScalarMake, ScalarMult, ScalarPrint, ScalarSub,
};
use crate::graph::nodes::struct_node::StructNode;
use crate::graph::nodes::traits::{IntoNodeInputPort, IntoNodeOutputPort};
use crate::graph::EditorGraphState;
use crate::value::draw::editor::EFieldEditor;
use crate::value::etype::registry::ETypesRegistry;
use crate::value::etype::EDataType;
use crate::value::EValue;
use crate::EditorGraph;
use camino::Utf8PathBuf;
use egui_node_graph::{
    Graph, InputParam, NodeId, NodeTemplateIter, NodeTemplateTrait, OutputParam,
};
use enum_dispatch::enum_dispatch;
use itertools::Itertools;
use rust_i18n::t;
use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};

pub mod traits;

pub mod data;
pub mod scalar;
pub mod struct_node;
pub mod vector;

#[enum_dispatch]
pub trait EditorNode {
    fn create_ports(
        &self,
        graph: &mut Graph<EditorNodeData, EDataType, EValue>,
        user_state: &mut EditorGraphState,
        node_id: NodeId,
    );

    fn categories(&self) -> Vec<&'static str>;

    fn has_side_effects(&self) -> bool;

    fn evaluate(
        &self,
        graph: &EditorGraph,
        outputs_cache: &mut OutputsCache,
        commands: &mut Vec<Command>,
        node_id: NodeId,
    ) -> anyhow::Result<()>;

    fn label(&self) -> Option<String> {
        None
    }

    fn appear_in_search(&self) -> bool {
        true
    }

    fn user_data(&self, _user_state: &mut EditorGraphState) -> Option<EditorNodeData> {
        None
    }

    fn sync_graph_data(&mut self, _user_state: &mut EditorGraphState) -> Option<NodeSyncData> {
        None
    }
}

/// NodeTemplate is a mechanism to define node templates. It's what the graph
/// will display in the "new node" popup. The user code needs to tell the
/// library how to convert a NodeTemplate into a Node.
#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, AsRefStr, EnumIter, PartialEq,
)]
#[enum_dispatch(EditorNode)]
pub enum NodeType {
    Struct(StructNode),
    // Scalar
    Scalar(ScalarMake),
    ScalarAdd(ScalarAdd),
    ScalarSub(ScalarSub),
    ScalarMult(ScalarMult),
    ScalarDiv(ScalarDiv),
    ScalarPrint(ScalarPrint),
    // Vec2
    // Vec2(Vec2Make),
    // Vec2Add(Vec2Add),
    // Vec2Sub(Vec2Sub),
    // Vec2Scale(Vec2Scale),
    // Vec2Print(Vec2Print),
}

impl NodeType {
    pub fn label(&self) -> String {
        EditorNode::label(self).unwrap_or_else(|| t!(self.as_ref()))
    }
}

// A trait for the node kinds, which tells the library how to build new nodes
// from the templates in the node finder
impl NodeTemplateTrait for NodeType {
    type NodeData = EditorNodeData;
    type DataType = EDataType;
    type ValueType = EValue;
    type UserState = EditorGraphState;
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

    fn user_data(&self, user_state: &mut Self::UserState) -> Self::NodeData {
        if let Some(data) = EditorNode::user_data(self, user_state) {
            data
        } else {
            EditorNodeData {
                template: *self,
                editors: Default::default(),
            }
        }
    }

    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
        node_id: NodeId,
    ) {
        EditorNode::create_ports(self, graph, user_state, node_id);
    }
}

pub struct AllEditorNodeTypes(pub Rc<RefCell<ETypesRegistry>>);
impl NodeTemplateIter for AllEditorNodeTypes {
    type Item = NodeType;

    fn all_kinds(&self) -> Vec<Self::Item> {
        let reg = self.0.borrow();
        let structs = reg.all_objects().filter_map(|e| e.as_struct()).map(|e| {
            NodeType::Struct(StructNode {
                ident: Some(e.ident),
            })
        });

        structs
            .chain(NodeType::iter().filter(EditorNode::appear_in_search))
            .collect_vec()
    }
}

pub fn create_input_port<T: IntoNodeInputPort>(
    graph: &mut Graph<EditorNodeData, EDataType, EValue>,
    user_state: &mut EditorGraphState,
    node_id: NodeId,
    name: String,
) {
    T::create_input_port(graph, user_state, node_id, name)
}

pub fn create_output_port<T: IntoNodeOutputPort>(
    graph: &mut Graph<EditorNodeData, EDataType, EValue>,
    user_state: &mut EditorGraphState,
    node_id: NodeId,
    name: String,
) {
    T::create_output_port(graph, user_state, node_id, name)
}

pub struct NodeSyncData {
    inputs: Vec<(Cow<'static, str>, InputParam<EDataType, EValue>)>,
    outputs: Vec<(Cow<'static, str>, OutputParam<EDataType>)>,
    editors: Vec<(Utf8PathBuf, Box<dyn EFieldEditor>)>,
}

impl NodeSyncData {
    pub fn new(
        inputs: Vec<(Cow<'static, str>, InputParam<EDataType, EValue>)>,
        outputs: Vec<(Cow<'static, str>, OutputParam<EDataType>)>,
        editors: Vec<(Utf8PathBuf, Box<dyn EFieldEditor>)>,
    ) -> Self {
        Self {
            inputs,
            outputs,
            editors,
        }
    }
}

pub fn sync_node_data(
    node_id: NodeId,
    graph: &mut Graph<EditorNodeData, EDataType, EValue>,
    user_state: &mut EditorGraphState,
) {
    let Graph {
        nodes,
        inputs: all_inputs,
        outputs: all_outputs,
        connections: all_connections,
        ..
    } = graph;

    let Some(node) = nodes.get_mut(node_id) else {
        return;
    };

    let Some(NodeSyncData {
        inputs,
        outputs,
        editors,
    }) = node.user_data.template.sync_graph_data(user_state)
    else {
        return;
    };

    node.user_data.editors.clear();
    node.user_data.editors.extend(editors);

    let mut old_inputs: FxHashMap<_, _> = std::mem::take(&mut node.inputs).into_iter().collect();
    for (name, mut input) in inputs {
        let key = if let Some(in_map) = old_inputs
            .remove(name.as_ref())
            .and_then(|k| all_inputs.get_mut(k))
        {
            let mut old = input;
            std::mem::swap(in_map, &mut old);
            in_map.id = old.id;
            in_map.node = old.node;
            in_map.value = old.value;
            in_map.id
        } else {
            input.node = node.id;
            all_inputs.insert_with_key(|id| {
                input.id = id;
                input
            })
        };
        node.inputs.push((name.to_string(), key));
    }

    for id in old_inputs.into_values() {
        all_connections.remove(id);
    }

    let mut old_outputs: FxHashMap<_, _> = std::mem::take(&mut node.outputs).into_iter().collect();
    for (name, mut output) in outputs {
        let key = if let Some(in_map) = old_outputs
            .remove(name.as_ref())
            .and_then(|k| all_outputs.get_mut(k))
        {
            let mut old = output;
            std::mem::swap(in_map, &mut old);
            in_map.id = old.id;
            in_map.node = old.node;
            in_map.id
        } else {
            output.node = node.id;
            all_outputs.insert_with_key(|id| {
                output.id = id;
                output
            })
        };
        node.outputs.push((name.to_string(), key));
    }

    let mut deleted_outputs: FxHashSet<_> = old_outputs.into_values().collect();
    if !deleted_outputs.is_empty() {
        for (_, conn) in all_connections {
            conn.retain(|o| !deleted_outputs.remove(o));
            if deleted_outputs.is_empty() {
                break;
            }
        }
    }
}

pub fn sync_all_nodes_data(
    graph: &mut Graph<EditorNodeData, EDataType, EValue>,
    user_state: &mut EditorGraphState,
) {
    for x in graph.nodes.keys().collect_vec() {
        sync_node_data(x, graph, user_state)
    }
}
