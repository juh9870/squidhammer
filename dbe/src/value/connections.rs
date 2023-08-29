// macro_rules! input_connection {
//     ($node_id: node_id) => {};
// }

use crate::value::etype::EDataType;
use crate::EditorGraph;
use egui_node_graph::{InputParamKind, NodeId};
use std::num::NonZeroU32;

#[inline(always)]
pub fn input_connection(node_id: NodeId, data_type: EDataType) -> impl Fn(&mut EditorGraph, &str) {
    move |graph: &mut EditorGraph, name: &str| {
        graph.add_input_param(
            node_id,
            name.to_string(),
            data_type,
            data_type.default_value(),
            InputParamKind::ConnectionOrConstant,
            true,
        );
    }
}
#[inline(always)]
pub fn wide_input_connection(
    node_id: NodeId,
    data_type: EDataType,
    max_connections: Option<NonZeroU32>,
) -> impl Fn(&mut EditorGraph, &str) {
    move |graph: &mut EditorGraph, name: &str| {
        graph.add_wide_input_param(
            node_id,
            name.to_string(),
            data_type,
            data_type.default_value(),
            InputParamKind::ConnectionOrConstant,
            max_connections,
            true,
        );
    }
}

#[inline(always)]
pub fn output_connection(node_id: NodeId, data_type: EDataType) -> impl Fn(&mut EditorGraph, &str) {
    move |graph: &mut EditorGraph, name: &str| {
        graph.add_output_param(node_id, name.to_string(), data_type);
    }
}
