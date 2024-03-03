use camino::Utf8PathBuf;
use itertools::Itertools;

use serde::{Deserialize, Serialize};

use std::borrow::Cow;
use std::num::NonZeroU32;

use egui_node_graph::{Graph, InputParam, InputParamKind, NodeId, OutputParam};

use crate::graph::commands::Command;
use crate::graph::evaluator::OutputsCache;
use crate::graph::nodes::data::EditorNodeData;
use crate::graph::nodes::{EditorNode, NodeSyncData};
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
        _graph: &mut Graph<EditorNodeData, EDataType, EValue>,
        _user_state: &mut EditorGraphState,
        _node_id: NodeId,
    ) {
    }

    fn categories(&self) -> Vec<&'static str> {
        vec!["structs"]
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

    fn sync_graph_data(&mut self, user_state: &mut EditorGraphState) -> Option<NodeSyncData> {
        let reg = user_state.registry.borrow();
        let Some((data, id)) = self.get_data(&reg) else {
            return None;
        };
        let input_parameters = data
            .fields
            .iter()
            .map(|field| {
                let is_const = matches!(field.ty, EItemType::Const(_));
                let p = InputParam {
                    id: Default::default(),
                    node: Default::default(),
                    max_connections: NonZeroU32::new(1),
                    value: field.ty.default_value(&reg),
                    typ: field.ty.ty(),

                    kind: if is_const {
                        InputParamKind::ConstantOnly
                    } else {
                        InputParamKind::ConnectionOrConstant
                    },
                    shown_inline: true,
                };
                (Cow::Borrowed(field.name.as_str()), p)
            })
            .collect_vec();

        let mut output_parameters = vec![];

        if let Some(f) = data.id_field_data() {
            output_parameters.push((
                Cow::Borrowed("Id"),
                OutputParam {
                    id: Default::default(),
                    node: Default::default(),
                    typ: EDataType::Ref { ty: f.ty },
                },
            ));
        } else {
            output_parameters.push((
                Cow::Borrowed("Data"),
                OutputParam {
                    id: Default::default(),
                    node: Default::default(),
                    typ: EDataType::Object { ident: id },
                },
            ));
        }

        let editors = data
            .fields
            .iter()
            .map(|field| {
                let field_name = field.name.as_str();
                let editor = reg.editor_for_or_err(field.ty.editor_name(), &field.ty);
                (Utf8PathBuf::from("/").join(field_name), editor)
            })
            .collect();

        Some(NodeSyncData::new(
            input_parameters,
            output_parameters,
            editors,
        ))
    }
}
