use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::groups::utils::{get_port_input, get_port_output, sync_fields};
use crate::graph::node::ports::fields::IoDirection;
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, Node, NodeContext, NodeFactory, SnarlNode};
use crate::graph::region::RegionVariable;
use crate::value::EValue;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use emath::{vec2, Pos2};
use inline_tweak::tweak;
use miette::bail;
use smallvec::SmallVec;
use std::fmt::Debug;
use std::marker::PhantomData;
use strum::EnumIs;
use ustr::Ustr;
use utils::vec_utils::VecOperation;
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

    fn update_state(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        id: NodeId,
    ) -> miette::Result<()> {
        let Some(region) = context.regions.get(&self.region) else {
            bail!("Region not found");
        };

        sync_fields(
            commands,
            &region.variables,
            &mut self.ids,
            None,
            id,
            IoDirection::Both,
        );

        Ok(())
    }

    fn inputs_count(&self, context: NodeContext) -> usize {
        self.ids.len() + self.node.inputs_count(context, self.kind) + 1
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
        if input == self.ids.len() + native_in_count {
            // special "new" input
            Ok(InputData::new(NodePortType::BasedOnSource, "".into()))
        } else {
            get_port_input(&region.variables, &self.ids, input - native_in_count)
        }
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

    fn try_connect(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<bool> {
        if to.id.input == self.inputs_count(context) - 1 {
            let new_pin_id = Uuid::new_v4();
            self.ids.push(new_pin_id);
            commands.push(SnarlCommand::EditRegionVariables {
                region: self.region,
                operation: VecOperation::Push(RegionVariable {
                    ty: Some(incoming_type.ty()),
                    id: new_pin_id,
                    name: "value".to_string(),
                }),
            })
        }

        self._default_try_connect(context, commands, from, to, incoming_type)
    }

    fn region_source(&self, _context: NodeContext) -> Option<Uuid> {
        self.kind.is_start().then_some(self.region)
    }

    fn region_end(&self, _context: NodeContext) -> Option<Uuid> {
        self.kind.is_end().then_some(self.region)
    }

    fn has_side_effects(&self) -> bool {
        self.kind.is_end()
    }

    fn should_execute_dependencies(
        &self,
        context: NodeContext,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        Ok(self.kind.is_start() || self.node.should_execute(context, self.region, variables)?)
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

#[derive(Debug)]
pub struct RegionalNodeFactory<T: RegionalNode>(PhantomData<T>);

impl<T: RegionalNode> RegionalNodeFactory<T> {
    pub const INSTANCE: Self = Self(PhantomData);
}

impl<T: RegionalNode> NodeFactory for RegionalNodeFactory<T> {
    fn id(&self) -> Ustr {
        T::id()
    }

    fn categories(&self) -> &'static [&'static str] {
        T::categories()
    }

    fn create(&self) -> Box<dyn Node> {
        Box::new(RegionIONode {
            region: Uuid::default(),
            kind: RegionIoKind::Start,
            node: T::create(),
            ids: vec![],
        })
    }

    fn create_nodes(&self, graph: &mut Snarl<SnarlNode>, pos: Pos2) -> SmallVec<[NodeId; 2]> {
        let region = Uuid::new_v4();
        [RegionIoKind::Start, RegionIoKind::End]
            .into_iter()
            .enumerate()
            .map(|(i, kind)| {
                graph.insert_node(
                    pos + vec2(i as f32 * tweak!(300.0), 0.0),
                    SnarlNode::new(Box::new(RegionIONode {
                        region,
                        kind,
                        node: T::create(),
                        ids: vec![],
                    })),
                )
            })
            .collect()
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

    /// Checks if the region should be executed at least once
    ///
    /// This is called for the endpoint node only. Start node is always executed
    fn should_execute(
        &self,
        context: NodeContext,
        region: Uuid,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool>;

    /// Executes the region io node
    ///
    /// If the region uses regional data, make sure to remove it once the
    /// region execution is finished, to avoid issues with nested looping
    /// regions
    fn execute(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult>;

    fn categories() -> &'static [&'static str];
    fn create() -> Self;
}
