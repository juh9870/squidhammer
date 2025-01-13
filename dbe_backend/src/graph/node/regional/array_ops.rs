use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::commands::SnarlCommands;
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::regional::{RegionIoKind, RegionalNode};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::{InPin, NodeId, OutPin};
use itertools::Itertools;
use miette::{bail, miette, IntoDiagnostic};
use std::fmt::Debug;
use std::ops::ControlFlow;
use ustr::Ustr;
use uuid::Uuid;

pub mod construct;
pub mod for_each;
pub mod macros;

#[derive(Debug)]
pub enum ArrayOpField<'a> {
    List(&'a Option<EDataType>),
    Value(&'a Option<EDataType>),
    Fixed(EDataType),
}

#[derive(Debug)]
pub enum ArrayOpFieldMut<'a> {
    List(&'a mut Option<EDataType>),
    Value(&'a mut Option<EDataType>),
    Fixed(EDataType),
}

impl<'a> ArrayOpFieldMut<'a> {
    pub fn as_ref(&self) -> ArrayOpField {
        match self {
            ArrayOpFieldMut::List(ty) => ArrayOpField::List(ty),
            ArrayOpFieldMut::Value(ty) => ArrayOpField::Value(ty),
            ArrayOpFieldMut::Fixed(ty) => ArrayOpField::Fixed(*ty),
        }
    }

    pub fn specify_from(
        &mut self,
        registry: &ETypesRegistry,
        incoming: &NodePortType,
    ) -> miette::Result<bool> {
        if !self.as_ref().can_specify_from(incoming)? {
            return Ok(false);
        }

        match self {
            Self::List(ty) => {
                if ty.is_some() {
                    bail!("List type already set");
                }
                let EDataType::List { id } = incoming.ty() else {
                    return Ok(false);
                };

                let list = registry
                    .get_list(&id)
                    .ok_or_else(|| miette!("Unknown list type: {}", id))?;

                **ty = Some(list.value_type);
            }
            Self::Value(ty) => {
                if ty.is_some() {
                    bail!("Value type already set");
                }
                **ty = Some(incoming.ty());
            }
            ArrayOpFieldMut::Fixed(_) => {
                bail!("Fixed type cannot be changed");
            }
        }
        Ok(true)
    }

    /// Overwrites the fields with the incoming types
    pub fn load_from(&mut self, incoming: Option<EDataType>) -> miette::Result<()> {
        match self {
            ArrayOpFieldMut::List(ty) => {
                **ty = incoming;
            }
            ArrayOpFieldMut::Value(ty) => {
                **ty = incoming;
            }
            ArrayOpFieldMut::Fixed(_) => {}
        }
        Ok(())
    }
}

impl<'a> ArrayOpField<'a> {
    pub fn is_specific(&self) -> bool {
        match self {
            ArrayOpField::List(ty) => ty.is_some(),
            ArrayOpField::Value(ty) => ty.is_some(),
            ArrayOpField::Fixed(_) => true,
        }
    }

    pub fn ty(&self, registry: &ETypesRegistry) -> EDataType {
        match self {
            ArrayOpField::List(ty) => registry.list_of(ty.unwrap_or_else(EDataType::null)),
            ArrayOpField::Value(ty) => ty.unwrap_or_else(EDataType::null),
            ArrayOpField::Fixed(ty) => *ty,
        }
    }

    pub fn ty_opt(&self) -> Option<EDataType> {
        match self {
            ArrayOpField::List(ty) => **ty,
            ArrayOpField::Value(ty) => **ty,
            ArrayOpField::Fixed(ty) => Some(*ty),
        }
    }

    pub fn can_specify_from(&self, incoming: &NodePortType) -> miette::Result<bool> {
        match self {
            ArrayOpField::List(ty) => {
                if ty.is_some() {
                    bail!("List type already set");
                }
                Ok(incoming.ty().is_list())
            }
            ArrayOpField::Value(ty) => {
                if ty.is_some() {
                    bail!("Value type already set");
                }
                Ok(true)
            }
            ArrayOpField::Fixed(_) => {
                bail!("Fixed type cannot be changed");
            }
        }
    }
}

pub trait ArrayOpRepeatNode: 'static + Debug + Clone + Send + Sync {
    fn id() -> Ustr;
    fn input_names(&self, kind: RegionIoKind) -> &[&str];
    fn output_names(&self, kind: RegionIoKind) -> &[&str];

    fn inputs(&self, kind: RegionIoKind) -> impl AsRef<[ArrayOpField]>;
    fn outputs(&self, kind: RegionIoKind) -> impl AsRef<[ArrayOpField]>;
    fn inputs_mut(&mut self, kind: RegionIoKind) -> impl AsMut<[ArrayOpFieldMut]>;
    fn outputs_mut(&mut self, kind: RegionIoKind) -> impl AsMut<[ArrayOpFieldMut]>;

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

impl<T: ArrayOpRepeatNode> RegionalNode for T {
    fn id() -> Ustr {
        T::id()
    }

    fn write_json(
        &self,
        _registry: &ETypesRegistry,
        kind: RegionIoKind,
    ) -> miette::Result<JsonValue> {
        <T as ArrayOpRepeatNode>::write_json(self, _registry, kind)
    }

    fn parse_json(
        &mut self,
        _registry: &ETypesRegistry,
        kind: RegionIoKind,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        <T as ArrayOpRepeatNode>::parse_json(self, _registry, kind, value)
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
        <T as ArrayOpRepeatNode>::should_execute(self, context, region, variables)
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
        <T as ArrayOpRepeatNode>::execute(self, context, kind, region, inputs, outputs, variables)
    }

    fn categories() -> &'static [&'static str] {
        <T as ArrayOpRepeatNode>::categories()
    }

    fn create() -> Self {
        <T as ArrayOpRepeatNode>::create()
    }
}
