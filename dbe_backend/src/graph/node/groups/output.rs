use crate::graph::inputs::GraphOutput;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::groups::utils::{
    get_graph_io_field, get_port_input, map_group_outputs, sync_fields,
};
use crate::graph::node::ports::fields::IoDirection;
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::{
    impl_serde_node, ExecutionExtras, ExecutionResult, Node, NodeContext, NodeFactory,
};
use crate::value::EValue;
use egui_snarl::{InPin, NodeId, OutPin};
use miette::miette;
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
        get_graph_io_field(context.outputs, &self.ids, index)
    }
}

impl Node for GroupOutputNode {
    impl_serde_node!();

    fn id(&self) -> Ustr {
        GroupOutputNodeFactory.id()
    }

    fn update_state(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        id: NodeId,
    ) -> miette::Result<()> {
        sync_fields(
            commands,
            context.outputs,
            &mut self.ids,
            None,
            id,
            IoDirection::Input(0),
        );

        debug_assert_eq!(
            self.ids,
            context.outputs.iter().map(|o| o.id).collect::<Vec<_>>()
        );

        Ok(())
    }

    fn inputs_count(&self, _context: NodeContext) -> usize {
        self.ids.len()
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        get_port_input(context.outputs, &self.ids, input)
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

    fn try_connect(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<bool> {
        let field = self
            .get_field(context, to.id.input)
            .ok_or_else(|| miette!("Output {} was deleted", to.id.input))?;

        if field.ty.is_none() {
            if !incoming_type.is_specific() {
                return Ok(false);
            }
            commands.push(SnarlCommand::SetGroupOutputType {
                id: field.id,
                ty: incoming_type.ty(),
            })
        }

        self._default_try_connect(context, commands, from, to, incoming_type)
    }

    fn has_side_effects(&self) -> bool {
        true
    }

    fn execute(
        &self,
        context: NodeContext,
        inputs: &[EValue],
        _outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        let mut group_out = Vec::with_capacity(context.outputs.len());

        map_group_outputs(
            context.registry,
            context.outputs,
            &self.ids,
            inputs,
            &mut group_out,
        )?;

        debug_assert_eq!(group_out.len(), self.inputs_count(context));

        variables.set_outputs(group_out)?;

        Ok(ExecutionResult::Done)
    }
}

#[derive(Debug, Clone)]
pub struct GroupOutputNodeFactory;

impl NodeFactory for GroupOutputNodeFactory {
    fn id(&self) -> Ustr {
        "group_output".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &["node groups"]
    }

    fn create(&self) -> Box<dyn Node> {
        Box::new(GroupOutputNode::new())
    }
}
