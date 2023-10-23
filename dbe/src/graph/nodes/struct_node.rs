use crate::graph::commands::Command;
use crate::graph::evaluator::OutputsCache;
use crate::graph::nodes::data::EditorNodeData;
use crate::graph::nodes::EditorNode;
use crate::graph::{EditorGraph, EditorGraphState};
use crate::value::etype::registry::eitem::EItemTypeTrait;
use crate::value::etype::registry::ETypeId;
use crate::value::etype::EDataType;
use crate::value::EValue;
use egui_node_graph::{Graph, InputParamKind, NodeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StructNode {
    pub ident: Option<ETypeId>,
}

impl EditorNode for StructNode {
    fn create_ports(
        &self,
        graph: &mut Graph<EditorNodeData, EDataType, EValue>,
        user_state: &mut EditorGraphState,
        node_id: NodeId,
    ) {
        let Some(id) = self.ident else {
            return;
        };
        let reg = user_state.registry.borrow();
        let Some(data) = reg.get_struct(&id) else {
            return;
        };

        for field in &data.fields {
            graph.add_input_param(
                node_id,
                field.name.to_string(),
                field.ty.ty(),
                field.ty.default_value(&reg),
                InputParamKind::ConnectionOrConstant,
                true,
            );
        }

        graph.add_output_param(node_id, "data".to_string(), EDataType::Object { ident: id });
        if let Some(f) = data.id_field_data() {
            graph.add_output_param(node_id, "id".to_string(), EDataType::Ref { ty: f.ty });
        }
    }

    fn categories(&self) -> Vec<&'static str> {
        return vec!["structs"];
    }

    fn has_side_effects(&self) -> bool {
        true
    }

    fn evaluate(
        &self,
        _graph: &EditorGraph,
        _outputs_cache: &mut OutputsCache,
        _commands: &mut Vec<Command>,
        _node_id: NodeId,
    ) -> anyhow::Result<()> {
        todo!()
    }

    fn label(&self) -> Option<String> {
        self.ident.map(|e| e.to_string())
    }
}
