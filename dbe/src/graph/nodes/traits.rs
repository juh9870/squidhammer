use crate::graph::nodes::EditorNodeData;
use crate::value::etype::EDataType;
use crate::value::{ENumber, EValue};
use crate::EditorGraphState;
use egui_node_graph::{Graph, InputParamKind, NodeId};
use smallvec::{Array, SmallVec};

pub trait EValueTypeAdapter {
    fn value_type() -> EDataType;

    fn input_kind() -> InputParamKind {
        InputParamKind::ConnectionOrConstant
    }
}

pub trait IntoNodeInputPort {
    fn create_input_port(
        graph: &mut Graph<EditorNodeData, EDataType, EValue>,
        user_state: &mut EditorGraphState,
        node_id: NodeId,
        name: String,
    );
}

pub trait IntoNodeOutputPort {
    fn create_output_port(
        graph: &mut Graph<EditorNodeData, EDataType, EValue>,
        user_state: &mut EditorGraphState,
        node_id: NodeId,
        name: String,
    );
}

impl<T: EValueTypeAdapter> IntoNodeInputPort for T {
    fn create_input_port(
        graph: &mut Graph<EditorNodeData, EDataType, EValue>,
        user_state: &mut EditorGraphState,
        node_id: NodeId,
        name: String,
    ) {
        graph.add_input_param(
            node_id,
            name,
            T::value_type(),
            T::value_type().default_value(&user_state.registry.borrow()),
            T::input_kind(),
            !matches![T::input_kind(), InputParamKind::ConnectionOnly],
        );
    }
}

impl<T: EValueTypeAdapter> IntoNodeOutputPort for T {
    fn create_output_port(
        graph: &mut Graph<EditorNodeData, EDataType, EValue>,
        _user_state: &mut EditorGraphState,
        node_id: NodeId,
        name: String,
    ) {
        graph.add_output_param(node_id, name, T::value_type());
    }
}

impl<T: EValueTypeAdapter> IntoNodeInputPort for Vec<T> {
    fn create_input_port(
        graph: &mut Graph<EditorNodeData, EDataType, EValue>,
        user_state: &mut EditorGraphState,
        node_id: NodeId,
        name: String,
    ) {
        graph.add_wide_input_param(
            node_id,
            name,
            T::value_type(),
            T::value_type().default_value(&user_state.registry.borrow()),
            T::input_kind(),
            None,
            !matches![T::input_kind(), InputParamKind::ConnectionOnly],
        );
    }
}
impl<T: EValueTypeAdapter, A: Array<Item = T>> IntoNodeInputPort for SmallVec<A> {
    fn create_input_port(
        graph: &mut Graph<EditorNodeData, EDataType, EValue>,
        user_state: &mut EditorGraphState,
        node_id: NodeId,
        name: String,
    ) {
        graph.add_wide_input_param(
            node_id,
            name,
            T::value_type(),
            T::value_type().default_value(&user_state.registry.borrow()),
            T::input_kind(),
            None,
            !matches![T::input_kind(), InputParamKind::ConnectionOnly],
        );
    }
}

impl EValueTypeAdapter for ENumber {
    fn value_type() -> EDataType {
        EDataType::Number
    }
}

impl EValueTypeAdapter for String {
    fn value_type() -> EDataType {
        EDataType::String
    }
}
