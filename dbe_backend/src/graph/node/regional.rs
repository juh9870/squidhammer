use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::groups::utils::{
    get_graph_io_field, get_port_input, get_port_output, sync_fields,
};
use crate::graph::node::ports::fields::IoDirection;
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, Node, NodeContext, NodeFactory, SnarlNode};
use crate::graph::region::RegionVariable;
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use emath::{vec2, Pos2};
use inline_tweak::tweak;
use miette::{bail, miette, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use serde_json::json;
use smallvec::SmallVec;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::ControlFlow;
use strum::EnumIs;
use ustr::Ustr;
use utils::vec_utils::VecOperation;
use uuid::Uuid;

pub mod generic_regional;
pub mod repeat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIs, Serialize, Deserialize)]
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

impl<T: RegionalNode> RegionIONode<T> {
    fn get_variable<'ctx>(
        &self,
        context: NodeContext<'ctx>,
        index: usize,
    ) -> Option<&'ctx RegionVariable> {
        let region = context.regions.get(&self.region)?;

        get_graph_io_field(&region.variables, &self.ids, index)
    }

    fn variables_length(&self) -> usize {
        if T::allow_variables() {
            self.ids.len()
        } else {
            0
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PackedRegionIoNode {
    region: Uuid,
    kind: RegionIoKind,
    node: JsonValue,
    ids: Vec<Uuid>,
}

impl<T: RegionalNode> Node for RegionIONode<T> {
    fn write_json(&self, registry: &ETypesRegistry) -> miette::Result<JsonValue> {
        let node = self.node.write_json(registry, self.kind)?;

        Ok(json! ({
            "region": self.region,
            "kind": self.kind,
            "node": node,
            "ids": self.ids.clone(),
        }))
    }

    fn parse_json(
        &mut self,
        registry: &ETypesRegistry,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        let mut packed: PackedRegionIoNode =
            serde_json::from_value(value.take()).into_diagnostic()?;
        self.region = packed.region;
        self.kind = packed.kind;
        self.ids = packed.ids;
        self.node
            .parse_json(registry, self.kind, &mut packed.node)?;

        Ok(())
    }

    fn id(&self) -> Ustr {
        T::id()
    }

    fn update_state(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        id: NodeId,
    ) -> miette::Result<()> {
        if !T::allow_variables() {
            return Ok(());
        }
        let Some(region) = context.regions.get(&self.region) else {
            bail!("Region not found");
        };

        sync_fields(
            commands,
            &region.variables,
            &mut self.ids,
            None,
            id,
            IoDirection::Both {
                input_offset: self.node.inputs_count(context, self.kind),
                output_offset: self.node.outputs_count(context, self.kind),
            },
        );

        Ok(())
    }

    fn inputs_count(&self, context: NodeContext) -> usize {
        self.variables_length() + self.node.inputs_count(context, self.kind) + 1
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        let native_in_count = self.node.inputs_count(context, self.kind);
        if input < self.node.inputs_count(context, self.kind) {
            return self.node.input_unchecked(context, self.kind, input);
        }

        if !T::allow_variables() {
            return Ok(InputData::new(
                NodePortType::Invalid,
                "!!unknown input!!".into(),
            ));
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
        self.variables_length() + self.node.outputs_count(context, self.kind)
    }

    fn output_unchecked(&self, context: NodeContext, output: usize) -> miette::Result<OutputData> {
        let native_out_count = self.node.outputs_count(context, self.kind);
        if output < native_out_count {
            return self.node.output_unchecked(context, self.kind, output);
        }

        if !T::allow_variables() {
            return Ok(OutputData::new(
                NodePortType::Invalid,
                "!!unknown input!!".into(),
            ));
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
        if to.id.input < self.node.inputs_count(context, self.kind) {
            if let ControlFlow::Break(value) = self.node.try_connect(
                context,
                self.kind,
                self.region,
                commands,
                from,
                to,
                incoming_type,
            )? {
                return Ok(value);
            };
        } else if T::allow_variables() {
            if to.id.input == self.inputs_count(context) - 1 {
                let new_pin_id = Uuid::new_v4();
                commands.push(SnarlCommand::EditRegionVariables {
                    region: self.region,
                    operation: VecOperation::Push(RegionVariable {
                        ty: Some(incoming_type.ty()),
                        id: new_pin_id,
                        name: "value".to_string(),
                    }),
                })
            } else {
                let input = to.id.input - self.node.inputs_count(context, self.kind);

                let field = self
                    .get_variable(context, input)
                    .ok_or_else(|| miette!("Variable {} is missing", input))?;

                if field.ty.is_none() {
                    if !incoming_type.is_specific() {
                        return Ok(false);
                    }
                    commands.push(SnarlCommand::EditRegionVariables {
                        region: self.region,
                        operation: VecOperation::Replace(
                            input,
                            RegionVariable {
                                ty: Some(incoming_type.ty()),
                                id: field.id,
                                name: field.name.clone(),
                            },
                        ),
                    });
                }
            }
        }

        self._default_try_connect(context, commands, from, to, incoming_type)
    }

    fn can_output_to(
        &self,
        context: NodeContext,
        from: &OutPin,
        to: &InPin,
        target_type: &NodePortType,
    ) -> miette::Result<bool> {
        if !T::allow_variables() {
            return Ok(false);
        }

        if from.id.output < self.node.outputs_count(context, self.kind) {
            return self
                .node
                .can_output_to(context, self.kind, self.region, from, to, target_type);
        }

        let Some(field) = self.get_variable(
            context,
            from.id.output - self.node.outputs_count(context, self.kind),
        ) else {
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
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<()> {
        // do NOT sync fields in this method, rearrangement of the fields might cause issues with pending connection commands

        if from.id.output < self.node.outputs_count(context, self.kind) {
            return self.node.connected_to_output(
                context,
                self.kind,
                self.region,
                commands,
                from,
                to,
                incoming_type,
            );
        }

        let output = from.id.output - self.node.outputs_count(context, self.kind);

        let field = self
            .get_variable(context, output)
            .expect("variable should exist, because `can_output_to` succeeded");

        if field.ty.is_some() {
            panic!("variable should not have a type, because `can_output_to` succeeded");
        };

        commands.push(SnarlCommand::EditRegionVariables {
            region: self.region,
            operation: VecOperation::Replace(
                output,
                RegionVariable {
                    ty: Some(incoming_type.ty()),
                    id: field.id,
                    name: field.name.clone(),
                },
            ),
        });

        Ok(())
    }

    fn region_source(&self) -> Option<Uuid> {
        self.kind.is_start().then_some(self.region)
    }

    fn region_end(&self) -> Option<Uuid> {
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

    /// Checks whether the region can have variables
    fn allow_variables() -> bool {
        true
    }

    /// Writes node state to json
    fn write_json(
        &self,
        registry: &ETypesRegistry,
        kind: RegionIoKind,
    ) -> miette::Result<JsonValue> {
        let _ = (registry, kind);
        Ok(JsonValue::Null)
    }
    /// Loads node state from json
    fn parse_json(
        &mut self,
        registry: &ETypesRegistry,
        kind: RegionIoKind,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        let _ = (registry, kind, value);
        Ok(())
    }

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

    #[allow(clippy::too_many_arguments)]
    fn try_connect(
        &mut self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<ControlFlow<bool>> {
        let _ = (context, kind, region, commands, from, to, incoming_type);
        Ok(ControlFlow::Continue(()))
    }

    /// Custom logic for checking if the node can output to the given port
    ///
    /// Only called if the corresponding output has type [NodePortType::BasedOnTarget]
    fn can_output_to(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        from: &OutPin,
        to: &InPin,
        target_type: &NodePortType,
    ) -> miette::Result<bool> {
        let _ = (context, kind, region, from, to, target_type);
        unimplemented!("Node::can_output_to")
    }

    /// Custom logic to be run after the output is connected to some input
    ///
    /// Only called if the corresponding output has type [NodePortType::BasedOnTarget]
    #[allow(clippy::too_many_arguments)]
    fn connected_to_output(
        &mut self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<()> {
        let _ = (context, kind, region, commands, from, to, incoming_type);
        unimplemented!("Node::can_output_to")
    }

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

fn remember_variables(
    state_values: &mut Option<Vec<EValue>>,
    inputs: &[EValue],
    outputs: &mut Vec<EValue>,
) {
    if let Some(values) = state_values.take() {
        outputs.extend(values);
    } else {
        outputs.extend(inputs.iter().cloned());
    }
}
