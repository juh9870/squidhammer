use crate::graph::node::groups::utils::{get_port_input, get_port_output};
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, Node, NodeContext};
use crate::value::EValue;
use std::fmt::Debug;
use strum::EnumIs;
use ustr::Ustr;
use uuid::Uuid;

pub mod repeat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIs)]
pub enum RegionIoKind {
    Start,
    End,
}

#[derive(Debug, Clone)]
pub struct RegionIONode<T: RegionalNode> {
    region: Uuid,
    kind: RegionIoKind,
    node: T,
    ids: Vec<Uuid>,
}

impl<T: RegionalNode> Node for RegionIONode<T> {
    fn id(&self) -> Ustr {
        T::id()
    }

    fn inputs_count(&self, context: NodeContext) -> usize {
        self.ids.len() + self.node.inputs_count(context, self.kind)
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        let native_in_count = self.node.inputs_count(context, self.kind);
        if input < self.node.inputs_count(context, self.kind) {
            return self.node.input_unchecked(context, self.kind, input);
        }

        let Some(region) = context.regions.get(&self.region) else {
            return Ok(InputData::new(
                NodePortType::Invalid,
                "!!unknown region!!".into(),
            ));
        };
        get_port_input(&region.variables, &self.ids, input - native_in_count)
    }

    fn outputs_count(&self, context: NodeContext) -> usize {
        self.ids.len() + self.node.outputs_count(context, self.kind)
    }

    fn output_unchecked(&self, context: NodeContext, output: usize) -> miette::Result<OutputData> {
        let native_out_count = self.node.outputs_count(context, self.kind);
        if output < native_out_count {
            return self.node.output_unchecked(context, self.kind, output);
        }

        let Some(region) = context.regions.get(&self.region) else {
            return Ok(OutputData::new(
                NodePortType::Invalid,
                "!!unknown region!!".into(),
            ));
        };
        get_port_output(&region.variables, &self.ids, output - native_out_count)
    }

    fn execute(
        &self,
        context: NodeContext,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        self.node
            .execute(context, self.kind, self.region, inputs, outputs, variables)
    }
}

pub trait RegionalNode: 'static + Debug + Clone + Send + Sync {
    fn id() -> Ustr;

    fn inputs_count(&self, context: NodeContext, kind: RegionIoKind) -> usize;
    fn outputs_count(&self, context: NodeContext, kind: RegionIoKind) -> usize;

    fn input_unchecked(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        input: usize,
    ) -> miette::Result<InputData>;

    fn output_unchecked(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        output: usize,
    ) -> miette::Result<OutputData>;

    fn execute(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult>;
}
