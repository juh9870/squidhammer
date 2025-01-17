use crate::graph::inputs::GraphInput;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::groups::utils::{
    get_graph_io_field, get_port_output, map_group_inputs, sync_fields,
};
use crate::graph::node::ports::fields::IoDirection;
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::serde_node::impl_serde_node;
use crate::graph::node::{ExecutionExtras, ExecutionResult, Node, NodeContext, NodeFactory};
use crate::value::EValue;
use egui_snarl::{InPin, NodeId, OutPin};
use serde::{Deserialize, Serialize};
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone, Hash, Default, Serialize, Deserialize)]
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
        get_graph_io_field(context.inputs, &self.ids, index)
    }
}

impl Node for GroupInputNode {
    impl_serde_node!();

    fn id(&self) -> Ustr {
        GroupInputNodeFactory.id()
    }

    fn update_state(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        id: NodeId,
    ) -> miette::Result<()> {
        sync_fields(
            commands,
            context.inputs,
            &mut self.ids,
            None,
            id,
            IoDirection::Output(0),
        );

        debug_assert_eq!(
            self.ids,
            context.inputs.iter().map(|o| o.id).collect::<Vec<_>>()
        );

        Ok(())
    }

    fn inputs_count(&self, _context: NodeContext) -> usize {
        0
    }

    fn input_unchecked(&self, _context: NodeContext, _input: usize) -> miette::Result<InputData> {
        panic!("GroupInputNode has no inputs")
    }

    fn outputs_count(&self, _context: NodeContext) -> usize {
        self.ids.len()
    }

    fn output_unchecked(&self, context: NodeContext, output: usize) -> miette::Result<OutputData> {
        get_port_output(context.inputs, &self.ids, output)
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

        commands.push(SnarlCommand::MarkDirty { node: from.id.node });

        Ok(())
    }

    fn execute(
        &self,
        context: NodeContext,
        _inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        let inputs = variables.get_inputs()?;

        map_group_inputs(context.registry, context.inputs, &self.ids, inputs, outputs)?;

        debug_assert_eq!(outputs.len(), self.outputs_count(context));

        Ok(ExecutionResult::Done)
    }
}

#[derive(Debug, Clone)]
pub struct GroupInputNodeFactory;

impl NodeFactory for GroupInputNodeFactory {
    fn id(&self) -> Ustr {
        "group_input".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &["node groups"]
    }

    fn create(&self) -> Box<dyn Node> {
        Box::new(GroupInputNode::new())
    }
}
