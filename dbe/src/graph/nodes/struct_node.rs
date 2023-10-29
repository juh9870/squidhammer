use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use egui_node_graph::{Graph, InputParamKind, NodeId};

use crate::graph::commands::Command;
use crate::graph::evaluator::OutputsCache;
use crate::graph::nodes::data::EditorNodeData;
use crate::graph::nodes::{EditorNode, NodeType};
use crate::graph::{EditorGraph, EditorGraphState};
use crate::value::etype::registry::eitem::{EItemType, EItemTypeTrait};
use crate::value::etype::registry::estruct::EStructData;
use crate::value::etype::registry::{ETypeId, ETypesRegistry};
use crate::value::etype::EDataType;
use crate::value::EValue;

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StructNode {
    pub ident: Option<ETypeId>,
}

impl StructNode {
    fn get_data<'a>(&self, reg: &'a ETypesRegistry) -> Option<(&'a EStructData, ETypeId)> {
        let Some(id) = self.ident else {
            return None;
        };
        let Some(data) = reg.get_struct(&id) else {
            return None;
        };
        Some((data, id))
    }
}

impl EditorNode for StructNode {
    fn create_ports(
        &self,
        graph: &mut Graph<EditorNodeData, EDataType, EValue>,
        user_state: &mut EditorGraphState,
        node_id: NodeId,
    ) {
        let reg = user_state.registry.borrow();
        let Some((data, id)) = self.get_data(&reg) else {
            return;
        };

        for field in &data.fields {
            let is_const = matches!(field.ty, EItemType::Const(_));
            graph.add_input_param(
                node_id,
                field.name.to_string(),
                field.ty.ty(),
                field.ty.default_value(&reg),
                if is_const {
                    InputParamKind::ConstantOnly
                } else {
                    InputParamKind::ConnectionOrConstant
                },
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
        self.ident.map(|ty| ty.to_string())
    }

    fn user_data(&self, user_state: &mut EditorGraphState) -> Option<EditorNodeData> {
        let reg = user_state.registry.borrow();
        let Some((data, ..)) = self.get_data(&reg) else {
            return None;
        };

        let editors = data
            .fields
            .iter()
            .map(|field| {
                let field_name = field.name.as_str();
                let editor = reg.editor_for_or_err(field.ty.editor_name(), &field.ty);
                (Utf8PathBuf::from("/").join(field_name), editor)
            })
            .collect();

        let data = EditorNodeData {
            template: NodeType::Struct(*self),
            editors,
        };

        Some(data)
    }
}
