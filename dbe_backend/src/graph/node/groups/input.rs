use crate::etype::eitem::EItemInfo;
use crate::graph::inputs::GraphInput;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::groups::utils::{get_field, sync_fields};
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::{
    impl_serde_node, ExecutionVariables, Node, NodeContext, NodeFactory, SnarlNode,
};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::{InPin, NodeId, OutPin};
use miette::bail;
use serde::{Deserialize, Serialize};
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GroupInputNode {
    pub ids: Vec<Uuid>,
}

impl GroupInputNode {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_field<'ctx>(
        &self,
        context: NodeContext<'ctx>,
        index: usize,
    ) -> Option<&'ctx GraphInput> {
        get_field(context.inputs, &self.ids, index)
    }
}

impl Node for GroupInputNode {
    impl_serde_node!();

    fn id(&self) -> Ustr {
        GroupInputNodeFactory.id()
    }

    fn title(&self, _context: NodeContext) -> String {
        "Group Input".to_string()
    }

    fn update_state(&mut self, context: NodeContext, commands: &mut SnarlCommands, id: NodeId) {
        sync_fields(commands, context.inputs, &mut self.ids, id);
    }

    fn inputs_count(&self, _context: NodeContext) -> usize {
        0
    }

    fn input_unchecked(&self, _context: NodeContext, _input: usize) -> miette::Result<InputData> {
        panic!("GroupInputNode has no inputs")
    }

    fn outputs_count(&self, context: NodeContext) -> usize {
        context.inputs.len()
    }

    fn output_unchecked(&self, context: NodeContext, output: usize) -> miette::Result<OutputData> {
        let Some(f) = self.get_field(context, output) else {
            return Ok(OutputData {
                ty: NodePortType::Invalid,
                name: "!!deleted input!!".into(),
            });
        };

        Ok(OutputData {
            ty: f
                .ty
                .map(EItemInfo::simple_type)
                .map(NodePortType::Specific)
                .unwrap_or_else(|| NodePortType::BasedOnTarget),
            name: f.name.as_str().into(),
        })
    }

    fn can_output_to(
        &self,
        context: NodeContext,
        from: &OutPin,
        _to: &InPin,
        _target_type: &NodePortType,
    ) -> miette::Result<bool> {
        let Some(field) = self.get_field(context, from.id.output) else {
            return Ok(false);
        };
        // This method getting called means that connection is attempted to the
        // `BasedOnInput` port, in which case we only allow it if the field has no type
        Ok(field.ty.is_none())
    }

    fn connected_to_output(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        from: &OutPin,
        _to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<()> {
        // do NOT sync fields in this method, rearrangement of the fields might cause issues with pending connection commands

        let field = self
            .get_field(context, from.id.output)
            .expect("output field should exist, because `can_output_to` succeeded");

        if field.ty.is_some() {
            panic!("output field should not have a type, because `can_output_to` succeeded");
        };

        commands.push(SnarlCommand::SetGroupInputType {
            ty: incoming_type.ty(),
            id: field.id,
        });

        Ok(())
    }

    fn execute(
        &self,
        context: NodeContext,
        _inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionVariables,
    ) -> miette::Result<()> {
        let inputs = variables.get_inputs()?;

        outputs.clear();

        // Fill the outputs with the input values in the order of the IDs
        for (i, id) in self.ids.iter().enumerate() {
            let input_pos = if context.inputs.get(i).is_some_and(|f| f.id == *id) {
                i
            } else if let Some(idx) = self.ids.iter().position(|f| f == id) {
                idx
            } else {
                bail!("Input {} was deleted", id);
            };

            outputs.push(inputs[input_pos].clone());
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GroupInputNodeFactory;

impl NodeFactory for GroupInputNodeFactory {
    fn id(&self) -> Ustr {
        "group_input".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &[]
    }

    fn create(&self) -> SnarlNode {
        Box::new(GroupInputNode::new())
    }
}
