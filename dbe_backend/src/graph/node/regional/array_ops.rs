use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::commands::SnarlCommands;
use crate::graph::node::generic::{GenericNodeField, GenericNodeFieldMut};
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::regional::{RegionIoKind, RegionalNode};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::{InPin, NodeId, OutPin};
use itertools::Itertools;
use miette::{bail, IntoDiagnostic};
use std::fmt::Debug;
use std::ops::ControlFlow;
use ustr::Ustr;
use uuid::Uuid;

pub mod construct;
pub mod for_each;

pub trait GenericRegionalNode: 'static + Debug + Clone + Send + Sync {
    fn id() -> Ustr;
    fn input_names(&self, kind: RegionIoKind) -> &[&str];
    fn output_names(&self, kind: RegionIoKind) -> &[&str];

    fn inputs(&self, kind: RegionIoKind) -> impl AsRef<[GenericNodeField]>;
    fn outputs(&self, kind: RegionIoKind) -> impl AsRef<[GenericNodeField]>;
    fn inputs_mut(&mut self, kind: RegionIoKind) -> impl AsMut<[GenericNodeFieldMut]>;
    fn outputs_mut(&mut self, kind: RegionIoKind) -> impl AsMut<[GenericNodeFieldMut]>;

    /// Writes node state to json
    fn write_json(
        &self,
        registry: &ETypesRegistry,
        kind: RegionIoKind,
    ) -> miette::Result<JsonValue> {
        let _ = (registry, kind);
        let inputs = self
            .inputs(kind)
            .as_ref()
            .iter()
            .map(|ty| ty.ty_opt())
            .collect_vec();
        let outputs = self
            .outputs(kind)
            .as_ref()
            .iter()
            .map(|ty| ty.ty_opt())
            .collect_vec();

        serde_json::to_value((inputs, outputs)).into_diagnostic()
    }

    /// Loads node state from json
    fn parse_json(
        &mut self,
        registry: &ETypesRegistry,
        kind: RegionIoKind,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        let _ = (registry, kind);
        let (inputs, outputs): (Vec<Option<EDataType>>, Vec<Option<EDataType>>) =
            serde_json::from_value(value.take()).into_diagnostic()?;

        for (ty, field) in inputs
            .into_iter()
            .zip(self.inputs_mut(kind).as_mut().iter_mut())
        {
            field.load_from(ty)?
        }
        for (ty, field) in outputs
            .into_iter()
            .zip(self.outputs_mut(kind).as_mut().iter_mut())
        {
            field.load_from(ty)?
        }

        Ok(())
    }

    /// Called after one of the node's inputs types has changed, allowing the
    /// node to update state of its pair
    fn state_changed(
        &mut self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        node: NodeId,
        commands: &mut SnarlCommands,
    ) {
        let _ = (context, kind, region, node, commands);
    }

    fn should_execute(
        &self,
        context: NodeContext,
        region: Uuid,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool>;

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

impl<T: GenericRegionalNode> RegionalNode for T {
    fn id() -> Ustr {
        T::id()
    }

    fn write_json(
        &self,
        _registry: &ETypesRegistry,
        kind: RegionIoKind,
    ) -> miette::Result<JsonValue> {
        <T as GenericRegionalNode>::write_json(self, _registry, kind)
    }

    fn parse_json(
        &mut self,
        _registry: &ETypesRegistry,
        kind: RegionIoKind,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        <T as GenericRegionalNode>::parse_json(self, _registry, kind, value)
    }

    fn inputs_count(&self, _context: NodeContext, kind: RegionIoKind) -> usize {
        self.inputs(kind).as_ref().len()
    }

    fn outputs_count(&self, _context: NodeContext, kind: RegionIoKind) -> usize {
        self.outputs(kind).as_ref().len()
    }

    fn input_unchecked(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        input: usize,
    ) -> miette::Result<InputData> {
        let inputs = self.inputs(kind);
        let Some(ty) = inputs.as_ref().get(input) else {
            bail!("Invalid input index: {}", input);
        };

        Ok(InputData::new(
            if ty.is_specific() {
                EItemInfo::simple_type(ty.ty(context.registry)).into()
            } else {
                NodePortType::BasedOnSource
            },
            self.input_names(kind)[input].into(),
        ))
    }

    fn output_unchecked(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        output: usize,
    ) -> miette::Result<OutputData> {
        let outputs = self.outputs(kind);
        let Some(ty) = outputs.as_ref().get(output) else {
            bail!("Invalid output index: {}", output);
        };

        Ok(OutputData::new(
            if ty.is_specific() {
                EItemInfo::simple_type(ty.ty(context.registry)).into()
            } else {
                NodePortType::BasedOnTarget
            },
            self.output_names(kind)[output].into(),
        ))
    }

    fn try_connect(
        &mut self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        commands: &mut SnarlCommands,
        _from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<ControlFlow<bool>> {
        let mut inputs = self.inputs_mut(kind);
        let Some(ty) = inputs.as_mut().get_mut(to.id.input) else {
            bail!("Invalid input index: {}", to.id.input);
        };

        if ty.as_ref().is_specific() {
            return Ok(ControlFlow::Continue(()));
        }

        if !ty.specify_from(context.registry, incoming_type)? {
            return Ok(ControlFlow::Break(false));
        }

        drop(inputs);

        self.state_changed(context, kind, region, to.id.node, commands);

        Ok(ControlFlow::Continue(()))
    }

    fn can_output_to(
        &self,
        _context: NodeContext,
        kind: RegionIoKind,
        _region: Uuid,
        from: &OutPin,
        _to: &InPin,
        target_type: &NodePortType,
    ) -> miette::Result<bool> {
        let outputs = self.outputs(kind);
        let Some(ty) = outputs.as_ref().get(from.id.output) else {
            bail!("Invalid output index: {}", from.id.output);
        };

        ty.can_specify_from(target_type)
    }

    fn connected_to_output(
        &mut self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        commands: &mut SnarlCommands,
        from: &OutPin,
        _to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<()> {
        let mut outputs = self.outputs_mut(kind);
        let Some(ty) = outputs.as_mut().get_mut(from.id.output) else {
            bail!("Invalid output index: {}", from.id.output);
        };

        if !ty.specify_from(context.registry, incoming_type)? {
            bail!("Failed to specify type");
        }

        drop(outputs);

        self.state_changed(context, kind, region, from.id.node, commands);

        Ok(())
    }

    fn should_execute(
        &self,
        context: NodeContext,
        region: Uuid,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        <T as GenericRegionalNode>::should_execute(self, context, region, variables)
    }

    fn execute(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        <T as GenericRegionalNode>::execute(self, context, kind, region, inputs, outputs, variables)
    }

    fn categories() -> &'static [&'static str] {
        <T as GenericRegionalNode>::categories()
    }

    fn create() -> Self {
        <T as GenericRegionalNode>::create()
    }
}
