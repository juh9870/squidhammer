use crate::nodes::MyNodeData;
use crate::value::etype::MyDataType;
use crate::value::{ENumber, EValue, EVector2};
use crate::MyGraphState;
use egui_node_graph::{Graph, InputParamKind, NodeId};
use smallvec::{Array, SmallVec};

pub trait EValueTypeAdapter {
    fn value_type() -> MyDataType;

    fn input_kind() -> InputParamKind {
        InputParamKind::ConnectionOrConstant
    }
}

pub trait IntoNodeInputPort {
    fn create_input_port(
        graph: &mut Graph<MyNodeData, MyDataType, EValue>,
        user_state: &mut MyGraphState,
        node_id: NodeId,
        name: String,
    );
}

pub trait IntoNodeOutputPort {
    fn create_output_port(
        graph: &mut Graph<MyNodeData, MyDataType, EValue>,
        user_state: &mut MyGraphState,
        node_id: NodeId,
        name: String,
    );
}

impl<T: EValueTypeAdapter> IntoNodeInputPort for T {
    fn create_input_port(
        graph: &mut Graph<MyNodeData, MyDataType, EValue>,
        _user_state: &mut MyGraphState,
        node_id: NodeId,
        name: String,
    ) {
        graph.add_input_param(
            node_id,
            name,
            T::value_type(),
            T::value_type().default_value(),
            T::input_kind(),
            !matches![T::input_kind(), InputParamKind::ConnectionOnly],
        );
    }
}

impl<T: EValueTypeAdapter> IntoNodeOutputPort for T {
    fn create_output_port(
        graph: &mut Graph<MyNodeData, MyDataType, EValue>,
        _user_state: &mut MyGraphState,
        node_id: NodeId,
        name: String,
    ) {
        graph.add_output_param(node_id, name, T::value_type());
    }
}

impl<T: EValueTypeAdapter> IntoNodeInputPort for Vec<T> {
    fn create_input_port(
        graph: &mut Graph<MyNodeData, MyDataType, EValue>,
        _user_state: &mut MyGraphState,
        node_id: NodeId,
        name: String,
    ) {
        graph.add_wide_input_param(
            node_id,
            name,
            T::value_type(),
            T::value_type().default_value(),
            T::input_kind(),
            None,
            !matches![T::input_kind(), InputParamKind::ConnectionOnly],
        );
    }
}
impl<T: EValueTypeAdapter, A: Array<Item = T>> IntoNodeInputPort for SmallVec<A> {
    fn create_input_port(
        graph: &mut Graph<MyNodeData, MyDataType, EValue>,
        _user_state: &mut MyGraphState,
        node_id: NodeId,
        name: String,
    ) {
        graph.add_wide_input_param(
            node_id,
            name,
            T::value_type(),
            T::value_type().default_value(),
            T::input_kind(),
            None,
            !matches![T::input_kind(), InputParamKind::ConnectionOnly],
        );
    }
}

impl EValueTypeAdapter for ENumber {
    fn value_type() -> MyDataType {
        MyDataType::Scalar
    }
}

impl EValueTypeAdapter for EVector2 {
    fn value_type() -> MyDataType {
        MyDataType::Vec2
    }
}
