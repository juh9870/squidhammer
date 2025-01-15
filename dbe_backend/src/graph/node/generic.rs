#![forbid(clippy::unconditional_recursion)]

use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::commands::SnarlCommands;
use crate::graph::node::editable_state::EditableState;
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, Node, NodeContext};
use crate::json_utils::JsonValue;
use crate::project::docs::{Docs, DocsWindowRef};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use downcast_rs::Downcast;
use dyn_clone::DynClone;
use egui_snarl::{InPin, NodeId, OutPin};
use itertools::Itertools;
use miette::{bail, miette, IntoDiagnostic};
use std::fmt::Debug;
use ustr::Ustr;

pub mod macros;

#[derive(Debug)]
pub enum GenericNodeField<'a> {
    List(&'a Option<EDataType>),
    Value(&'a Option<EDataType>),
    Fixed(EDataType),
}

#[derive(Debug)]
pub enum GenericNodeFieldMut<'a> {
    List(&'a mut Option<EDataType>),
    Value(&'a mut Option<EDataType>),
    Fixed(EDataType),
}

impl<'a> GenericNodeFieldMut<'a> {
    pub fn as_ref(&self) -> GenericNodeField {
        match self {
            GenericNodeFieldMut::List(ty) => GenericNodeField::List(ty),
            GenericNodeFieldMut::Value(ty) => GenericNodeField::Value(ty),
            GenericNodeFieldMut::Fixed(ty) => GenericNodeField::Fixed(*ty),
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
            GenericNodeFieldMut::Fixed(_) => {
                bail!("Fixed type cannot be changed");
            }
        }
        Ok(true)
    }

    /// Overwrites the fields with the incoming types
    pub fn load_from(&mut self, incoming: Option<EDataType>) -> miette::Result<()> {
        match self {
            GenericNodeFieldMut::List(ty) => {
                **ty = incoming;
            }
            GenericNodeFieldMut::Value(ty) => {
                **ty = incoming;
            }
            GenericNodeFieldMut::Fixed(_) => {}
        }
        Ok(())
    }
}

impl<'a> GenericNodeField<'a> {
    pub fn is_specific(&self) -> bool {
        match self {
            GenericNodeField::List(ty) => ty.is_some(),
            GenericNodeField::Value(ty) => ty.is_some(),
            GenericNodeField::Fixed(_) => true,
        }
    }

    pub fn ty(&self, registry: &ETypesRegistry) -> EDataType {
        match self {
            GenericNodeField::List(ty) => registry.list_of(ty.unwrap_or_else(EDataType::null)),
            GenericNodeField::Value(ty) => ty.unwrap_or_else(EDataType::null),
            GenericNodeField::Fixed(ty) => *ty,
        }
    }

    pub fn ty_opt(&self) -> Option<EDataType> {
        match self {
            GenericNodeField::List(ty) => **ty,
            GenericNodeField::Value(ty) => **ty,
            GenericNodeField::Fixed(ty) => Some(*ty),
        }
    }

    pub fn can_specify_from(&self, incoming: &NodePortType) -> miette::Result<bool> {
        match self {
            GenericNodeField::List(ty) => {
                if ty.is_some() {
                    bail!("List type already set");
                }
                Ok(incoming.ty().is_list())
            }
            GenericNodeField::Value(ty) => {
                if ty.is_some() {
                    bail!("Value type already set");
                }
                Ok(true)
            }
            GenericNodeField::Fixed(_) => {
                bail!("Fixed type cannot be changed");
            }
        }
    }
}

pub trait GenericNode: DynClone + Debug + Send + Sync + Downcast + 'static {
    fn write_json(&self, registry: &ETypesRegistry) -> miette::Result<JsonValue> {
        let _ = (registry,);
        write_generic_json_fields(self)
    }

    fn parse_json(
        &mut self,
        registry: &ETypesRegistry,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        let _ = (registry,);
        parse_generic_json_fields(self, value)?;
        Ok(())
    }

    /// Called after one of the node's inputs/output types has changed,
    /// allowing the node to update its state
    fn types_changed(&mut self, context: NodeContext, node: NodeId, commands: &mut SnarlCommands) {
        let _ = (context, node, commands);
    }

    fn id(&self) -> Ustr;
    fn input_names(&self) -> &[&str];
    fn output_names(&self) -> &[&str];

    fn inputs(&self) -> impl AsRef<[GenericNodeField]>;
    fn outputs(&self) -> impl AsRef<[GenericNodeField]>;
    fn inputs_mut(&mut self) -> impl AsMut<[GenericNodeFieldMut]>;
    fn outputs_mut(&mut self) -> impl AsMut<[GenericNodeFieldMut]>;

    fn title(&self, context: NodeContext, docs: &Docs) -> String {
        let _ = (context, docs);
        DocsWindowRef::Node(self.id())
            .title(docs, context.registry)
            .to_string()
    }

    /// See [Node::update_state]
    fn update_state(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        id: NodeId,
    ) -> miette::Result<()> {
        let _ = (context, commands, id);
        Ok(())
    }

    /// See [Node::has_editable_state]
    fn has_editable_state(&self) -> bool {
        false
    }

    /// See [Node::editable_state]
    fn editable_state(&self) -> EditableState {
        assert!(
            self.has_editable_state(),
            "editable_state should only be called if has_editable_state returns true"
        );
        unimplemented!()
    }

    /// See [Node::apply_editable_state]
    fn apply_editable_state(
        &mut self,
        state: EditableState,
        commands: &mut SnarlCommands,
        node_id: NodeId,
    ) -> miette::Result<()> {
        let _ = (state, commands, node_id);
        assert!(
            self.has_editable_state(),
            "apply_editable_state should only be called if has_editable_state returns true"
        );
        unimplemented!()
    }

    /// See [Node::has_inline_values]
    fn has_inline_values(&self) -> miette::Result<bool> {
        Ok(true)
    }

    /// See [Node::has_side_effects]
    fn has_side_effects(&self) -> bool {
        false
    }

    /// See [Node::should_execute_dependencies]
    fn should_execute_dependencies(
        &self,
        context: NodeContext,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        let _ = (context, variables);
        Ok(true)
    }

    /// See [Node::execute]
    fn execute(
        &self,
        context: NodeContext,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult>;
}

fn write_generic_json_fields<T: GenericNode + ?Sized>(node: &T) -> miette::Result<JsonValue> {
    let inputs = node
        .inputs()
        .as_ref()
        .iter()
        .map(|ty| ty.ty_opt())
        .collect_vec();
    let outputs = node
        .outputs()
        .as_ref()
        .iter()
        .map(|ty| ty.ty_opt())
        .collect_vec();

    serde_json::to_value((inputs, outputs)).into_diagnostic()
}

fn parse_generic_json_fields<T: GenericNode + ?Sized>(
    node: &mut T,
    value: &mut JsonValue,
) -> miette::Result<()> {
    let (inputs, outputs): (Vec<Option<EDataType>>, Vec<Option<EDataType>>) =
        serde_json::from_value(value.take()).into_diagnostic()?;

    for (ty, field) in inputs
        .into_iter()
        .zip(node.inputs_mut().as_mut().iter_mut())
    {
        field.load_from(ty)?
    }
    for (ty, field) in outputs
        .into_iter()
        .zip(node.outputs_mut().as_mut().iter_mut())
    {
        field.load_from(ty)?
    }

    Ok(())
}

impl<T: GenericNode> Node for T {
    fn write_json(&self, registry: &ETypesRegistry) -> miette::Result<JsonValue> {
        T::write_json(self, registry)
    }

    fn parse_json(
        &mut self,
        registry: &ETypesRegistry,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        T::parse_json(self, registry, value)
    }

    fn id(&self) -> Ustr {
        T::id(self)
    }

    fn update_state(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        id: NodeId,
    ) -> miette::Result<()> {
        T::update_state(self, context, commands, id)
    }

    fn has_editable_state(&self) -> bool {
        T::has_editable_state(self)
    }

    fn editable_state(&self) -> EditableState {
        T::editable_state(self)
    }

    fn apply_editable_state(
        &mut self,
        state: EditableState,
        commands: &mut SnarlCommands,
        node_id: NodeId,
    ) -> miette::Result<()> {
        T::apply_editable_state(self, state, commands, node_id)
    }

    fn has_inline_values(&self) -> miette::Result<bool> {
        T::has_inline_values(self)
    }

    fn inputs_count(&self, _context: NodeContext) -> usize {
        self.inputs().as_ref().len()
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        let inputs = self.inputs();
        let Some(ty) = inputs.as_ref().get(input) else {
            bail!("Invalid input index: {}", input);
        };

        Ok(InputData::new(
            if ty.is_specific() {
                EItemInfo::simple_type(ty.ty(context.registry)).into()
            } else {
                NodePortType::BasedOnSource
            },
            self.input_names()[input].into(),
        ))
    }

    fn outputs_count(&self, _context: NodeContext) -> usize {
        self.outputs().as_ref().len()
    }

    fn output_unchecked(&self, context: NodeContext, output: usize) -> miette::Result<OutputData> {
        let outputs = self.outputs();
        let Some(ty) = outputs.as_ref().get(output) else {
            bail!("Invalid output index: {}", output);
        };

        Ok(OutputData::new(
            if ty.is_specific() {
                EItemInfo::simple_type(ty.ty(context.registry)).into()
            } else {
                NodePortType::BasedOnTarget
            },
            self.output_names()[output].into(),
        ))
    }

    fn try_connect(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<bool> {
        {
            let mut inputs = self.inputs_mut();
            let Some(ty) = inputs.as_mut().get_mut(to.id.input) else {
                bail!("Invalid input index: {}", to.id.input);
            };

            if !ty.as_ref().is_specific() {
                if !ty.specify_from(context.registry, incoming_type)? {
                    return Ok(false);
                }

                drop(inputs);

                self.types_changed(context, to.id.node, commands);
            }
        }

        self._default_try_connect(context, commands, from, to, incoming_type)
    }

    fn can_output_to(
        &self,
        _context: NodeContext,
        from: &OutPin,
        _to: &InPin,
        target_type: &NodePortType,
    ) -> miette::Result<bool> {
        let outputs = self.outputs();
        let Some(ty) = outputs.as_ref().get(from.id.output) else {
            bail!("Invalid output index: {}", from.id.output);
        };

        ty.can_specify_from(target_type)
    }

    fn connected_to_output(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        from: &OutPin,
        _to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<()> {
        let mut outputs = self.outputs_mut();
        let Some(ty) = outputs.as_mut().get_mut(from.id.output) else {
            bail!("Invalid output index: {}", from.id.output);
        };

        if !ty.specify_from(context.registry, incoming_type)? {
            bail!("Failed to specify type");
        }

        drop(outputs);

        self.types_changed(context, from.id.node, commands);

        Ok(())
    }

    fn has_side_effects(&self) -> bool {
        todo!()
    }

    fn should_execute_dependencies(
        &self,
        context: NodeContext,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        T::should_execute_dependencies(self, context, variables)
    }

    fn execute(
        &self,
        context: NodeContext,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        T::execute(self, context, inputs, outputs, variables)
    }
}
