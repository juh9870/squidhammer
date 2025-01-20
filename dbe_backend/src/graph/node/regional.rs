use crate::etype::EDataType;
use crate::graph::inputs::GraphIoData;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::editable_state::EditableState;
use crate::graph::node::generic::{
    generic_can_output_to, generic_connected_to_output, generic_try_connect, GenericNodeField,
    GenericNodeFieldMut,
};
use crate::graph::node::groups::utils::{get_graph_io_field, sync_fields};
use crate::graph::node::ports::fields::IoDirection;
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::stateful::StatefulNode;
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, Node, NodeContext, NodeFactory, SnarlNode};
use crate::graph::region::RegionVariable;
use crate::json_utils::json_serde::JsonSerde;
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use bitflags::bitflags;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use emath::{vec2, Pos2};
use inline_tweak::tweak;
use miette::{bail, miette, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use serde_json::json;
use smallvec::SmallVec;
use std::fmt::Debug;
use std::hash::Hash;
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

bitflags! {
    pub struct RegionVariableSide: u8 {
        const START_IN  = 0b0001;
        const START_OUT = 0b0010;
        const END_IN    = 0b0100;
        const END_OUT   = 0b1000;
    }
}

#[derive(Debug, Clone, Hash)]
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

    fn allow_input_variables(&self) -> bool {
        match self.kind {
            RegionIoKind::Start => T::allow_variables().contains(RegionVariableSide::START_IN),
            RegionIoKind::End => T::allow_variables().contains(RegionVariableSide::END_IN),
        }
    }

    fn allow_output_variables(&self) -> bool {
        match self.kind {
            RegionIoKind::Start => T::allow_variables().contains(RegionVariableSide::START_OUT),
            RegionIoKind::End => T::allow_variables().contains(RegionVariableSide::END_OUT),
        }
    }

    fn input_variables_length(&self) -> usize {
        if self.allow_input_variables() {
            self.ids.len()
        } else {
            0
        }
    }

    fn output_variables_length(&self) -> usize {
        if self.allow_output_variables() {
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

        Ok(json!({
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
        let allow_in = self.allow_input_variables();
        let allow_out = self.allow_output_variables();
        if !allow_in && !allow_out {
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
            if allow_in && allow_out {
                IoDirection::Both {
                    input_offset: self.node.inputs_count(context, self.kind),
                    output_offset: self.node.outputs_count(context, self.kind),
                }
            } else if allow_in {
                IoDirection::Input(self.node.inputs_count(context, self.kind))
            } else {
                IoDirection::Output(self.node.outputs_count(context, self.kind))
            },
        );

        Ok(())
    }

    fn has_editable_state(&self) -> bool {
        self.node.has_editable_state(self.kind)
    }

    fn editable_state(&self) -> EditableState {
        self.node.editable_state(self.kind)
    }

    fn apply_editable_state(
        &mut self,
        context: NodeContext,
        state: EditableState,
        commands: &mut SnarlCommands,
        node_id: NodeId,
    ) -> miette::Result<()> {
        self.node
            .apply_editable_state(context, self.kind, state, commands, node_id)
    }

    fn inputs_count(&self, context: NodeContext) -> usize {
        self.input_variables_length()
            + self.node.inputs_count(context, self.kind)
            + if self.allow_input_variables() { 1 } else { 0 }
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        let native_in_count = self.node.inputs_count(context, self.kind);
        if input < self.node.inputs_count(context, self.kind) {
            return self.node.input_unchecked(context, self.kind, input);
        }

        if !self.allow_input_variables() {
            return Ok(InputData::invalid("unknown input"));
        }

        let Some(region) = context.regions.get(&self.region) else {
            return Ok(InputData::invalid("unknown region"));
        };
        if input == self.ids.len() + native_in_count {
            // special "new" input
            Ok(InputData::new(NodePortType::BasedOnSource, "".into()))
        } else {
            let Some(field) =
                get_graph_io_field(&region.variables, &self.ids, input - native_in_count)
            else {
                return Ok(InputData::invalid("unknown input"));
            };
            Ok(self
                .node
                .input_variable_type(context, self.kind, &field.ty)
                .as_input_ty(context, field.name.clone()))
        }
    }

    fn outputs_count(&self, context: NodeContext) -> usize {
        self.output_variables_length() + self.node.outputs_count(context, self.kind)
    }

    fn output_unchecked(&self, context: NodeContext, output: usize) -> miette::Result<OutputData> {
        let native_out_count = self.node.outputs_count(context, self.kind);
        if output < native_out_count {
            return self.node.output_unchecked(context, self.kind, output);
        }

        if !self.allow_output_variables() {
            return Ok(OutputData::invalid("!!unknown input!!"));
        }

        let Some(region) = context.regions.get(&self.region) else {
            return Ok(OutputData::invalid("!!unknown region!!"));
        };
        let Some(field) =
            get_graph_io_field(&region.variables, &self.ids, output - native_out_count)
        else {
            return Ok(OutputData::invalid("unknown input"));
        };
        Ok(self
            .node
            .output_variable_type(context, self.kind, &field.ty)
            .as_output_ty(context, field.name.clone()))
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
        } else if self.allow_input_variables() {
            if to.id.input == self.inputs_count(context) - 1 {
                let mut ty = None;
                let mut inputs = [self
                    .node
                    .input_variable_type_mut(context, self.kind, &mut ty)];
                match generic_try_connect(context, 0, incoming_type, inputs.as_mut_slice())? {
                    ControlFlow::Continue(changed) => {
                        assert!(
                            changed,
                            "generic_try_connect should succeed with changing ty, \
                            since incoming type is not specific"
                        )
                    }
                    ControlFlow::Break(_) => return Ok(false),
                };
                if let Some(ty) = ty {
                    let new_pin_id = Uuid::new_v4();
                    commands.push(SnarlCommand::EditRegionVariables {
                        region: self.region,
                        operation: VecOperation::Push(RegionVariable {
                            ty: Some(ty),
                            id: new_pin_id,
                            name: "value".to_string(),
                        }),
                    })
                }
            } else {
                let input = to.id.input - self.node.inputs_count(context, self.kind);

                let field = self
                    .get_variable(context, input)
                    .ok_or_else(|| miette!("Variable {} is missing", input))?;

                if field.ty.is_none() {
                    let mut ty = field.ty;
                    let mut inputs = [self
                        .node
                        .input_variable_type_mut(context, self.kind, &mut ty)];
                    let changed = match generic_try_connect(
                        context,
                        0,
                        incoming_type,
                        inputs.as_mut_slice(),
                    )? {
                        ControlFlow::Continue(changed) => changed,
                        ControlFlow::Break(_) => return Ok(false),
                    };

                    if let Some(ty) = ty {
                        commands.push(SnarlCommand::EditRegionVariables {
                            region: self.region,
                            operation: VecOperation::Replace(
                                input,
                                RegionVariable {
                                    ty: Some(ty),
                                    id: field.id,
                                    name: field.name.clone(),
                                },
                            ),
                        });
                    } else {
                        assert!(
                            !changed,
                            "ty should be Some if generic_try_connect returned true"
                        );
                    }
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
        if from.id.output < self.node.outputs_count(context, self.kind) {
            return self
                .node
                .can_output_to(context, self.kind, self.region, from, to, target_type);
        }

        if !self.allow_output_variables() {
            return Ok(false);
        }

        let Some(field) = self.get_variable(
            context,
            from.id.output - self.node.outputs_count(context, self.kind),
        ) else {
            return Ok(false);
        };

        let ty = field.ty();

        let outputs = [self.node.output_variable_type(context, self.kind, &ty)];
        generic_can_output_to(context, 0, target_type, &outputs)
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

        let mut ty = field.ty();
        let mut outputs = [self
            .node
            .output_variable_type_mut(context, self.kind, &mut ty)];
        let changed =
            generic_connected_to_output(context, 0, incoming_type, outputs.as_mut_slice())?;

        if let Some(ty) = ty {
            commands.push(SnarlCommand::EditRegionVariables {
                region: self.region,
                operation: VecOperation::Replace(
                    output,
                    RegionVariable {
                        ty: Some(ty),
                        id: field.id,
                        name: field.name.clone(),
                    },
                ),
            });
            Ok(())
        } else {
            assert!(
                !changed,
                "ty should be Some if generic_connected_to_output returned true"
            );
            Ok(())
        }
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

pub trait NodeWithVariables {
    /// Indicates at which sides the node can have variables
    fn allow_variables() -> RegionVariableSide {
        RegionVariableSide::all()
    }

    fn input_variable_type<'a>(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        ty: &'a Option<EDataType>,
    ) -> GenericNodeField<'a> {
        let _ = (context, kind);
        GenericNodeField::Value(ty)
    }

    fn output_variable_type<'a>(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        ty: &'a Option<EDataType>,
    ) -> GenericNodeField<'a> {
        let _ = (context, kind);
        GenericNodeField::Value(ty)
    }

    fn input_variable_type_mut<'a>(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a> {
        let _ = (context, kind);
        GenericNodeFieldMut::Value(ty)
    }

    fn output_variable_type_mut<'a>(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a> {
        let _ = (context, kind);
        GenericNodeFieldMut::Value(ty)
    }
}

pub trait RegionalNode:
    for<'a> StatefulNode<State<'a> = RegionIoKind>
    + for<'a> JsonSerde<State<'a> = RegionIoKind>
    + NodeWithVariables
{
}

impl<
        T: for<'a> StatefulNode<State<'a> = RegionIoKind>
            + for<'a> JsonSerde<State<'a> = RegionIoKind>
            + NodeWithVariables,
    > RegionalNode for T
{
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
