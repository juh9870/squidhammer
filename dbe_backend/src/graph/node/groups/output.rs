use crate::etype::eitem::EItemInfo;
use crate::graph::inputs::GraphOutput;
use crate::graph::node::commands::SnarlCommands;
use crate::graph::node::groups::utils::{get_field, sync_fields};
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::{
    impl_serde_node, ExecutionVariables, Node, NodeContext, NodeFactory, SnarlNode,
};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::NodeId;
use miette::bail;
use serde::{Deserialize, Serialize};
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GroupOutputNode {
    pub ids: Vec<Uuid>,
}

impl GroupOutputNode {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_field<'ctx>(
        &self,
        context: NodeContext<'ctx>,
        index: usize,
    ) -> Option<&'ctx GraphOutput> {
        get_field(context.outputs, &self.ids, index)
    }
}

impl Node for GroupOutputNode {
    impl_serde_node!();

    fn id(&self) -> Ustr {
        GroupOutputNodeFactory.id()
    }

    fn title(&self, _context: NodeContext) -> String {
        "Group Output".to_string()
    }

    fn update_state(&mut self, context: NodeContext, commands: &mut SnarlCommands, id: NodeId) {
        sync_fields(commands, context.outputs, &mut self.ids, id);
    }

    fn inputs_count(&self, context: NodeContext) -> usize {
        context.outputs.len()
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        let Some(f) = self.get_field(context, input) else {
            return Ok(InputData {
                ty: NodePortType::Invalid,
                name: "!!deleted output!!".into(),
            });
        };

        Ok(InputData {
            ty: f
                .ty
                .map(EItemInfo::simple_type)
                .map(NodePortType::Specific)
                .unwrap_or_else(|| NodePortType::BasedOnSource),
            name: f.name.as_str().into(),
        })
    }

    fn outputs_count(&self, _context: NodeContext) -> usize {
        0
    }

    fn output_unchecked(
        &self,
        _context: NodeContext,
        _output: usize,
    ) -> miette::Result<OutputData> {
        panic!("GroupOutputNode has no outputs")
    }

    fn execute(
        &self,
        context: NodeContext,
        inputs: &[EValue],
        _outputs: &mut Vec<EValue>,
        variables: &mut ExecutionVariables,
    ) -> miette::Result<()> {
        let mut group_out = Vec::with_capacity(context.outputs.len());

        // Fill the group outputs with the incoming values, matching the order of the IDs
        // New outputs will be filled with default values
        for (i, field) in context.outputs.iter().enumerate() {
            let Some(input_pos) = (if self.ids.get(i).is_some_and(|id| id == &field.id) {
                Some(i)
            } else {
                self.ids.iter().position(|f| f == &field.id)
            }) else {
                let default = field
                    .ty
                    .map(|f| f.default_value(context.registry).into_owned())
                    .unwrap_or_else(|| EValue::Null);
                group_out.push(default);
                continue;
            };

            group_out.push(inputs[input_pos].clone());
        }

        // Check is any output was removed and incoming value now has no matching output
        for (i, id) in self.ids.iter().enumerate() {
            if context.outputs.get(i).is_some_and(|f| f.id == *id) {
                continue;
            }
            if context.outputs.iter().any(|f| f.id == *id) {
                continue;
            }

            bail!("Output {} was deleted", id);
        }

        variables.set_outputs(group_out)?;

        // for (i, input) in inputs.iter().enumerate() {
        //     let Some(f) = self.get_field(context, i) else {
        //         bail!("Output {} is missing", i);
        //     };
        //
        //     variables.set(f.id, input.clone());
        // }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GroupOutputNodeFactory;

impl NodeFactory for GroupOutputNodeFactory {
    fn id(&self) -> Ustr {
        "group_output".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &[]
    }

    fn create(&self) -> SnarlNode {
        Box::new(GroupOutputNode::new())
    }
}
