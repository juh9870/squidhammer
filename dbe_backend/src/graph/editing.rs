use crate::graph::cache::GraphCache;
use crate::graph::execution::GraphExecutionContext;
use crate::graph::inputs::{GraphInput, GraphOutput};
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::enum_node::EnumNode;
use crate::graph::node::list::ListNode;
use crate::graph::node::struct_node::StructNode;
use crate::graph::node::{get_snarl_node, SnarlNode};
use crate::graph::Graph;
use crate::m_try;
use crate::project::side_effects::SideEffectsContext;
use crate::registry::{EObjectType, ETypesRegistry};
use crate::value::id::{EListId, ETypeId};
use crate::value::EValue;
use ahash::AHashMap;
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};
use emath::Pos2;
use miette::Context;
use smallvec::SmallVec;
use std::collections::hash_map::Entry;
use ustr::Ustr;

#[derive(Debug)]
pub struct GraphEditingContext<'a, 'snarl> {
    pub snarl: &'snarl mut Snarl<SnarlNode>,
    pub inline_values: &'a mut AHashMap<InPinId, EValue>,
    pub inputs: &'a mut SmallVec<[GraphInput; 1]>,
    pub outputs: &'a mut SmallVec<[GraphOutput; 1]>,
    pub registry: &'a ETypesRegistry,
    side_effects: SideEffectsContext<'a>,
    cache: &'a mut GraphCache,
}

impl<'a> GraphEditingContext<'a, 'a> {
    pub fn from_graph(
        graph: &'a mut Graph,
        registry: &'a ETypesRegistry,
        cache: &'a mut GraphCache,
        side_effects: SideEffectsContext<'a>,
    ) -> Self {
        GraphEditingContext {
            snarl: &mut graph.snarl,
            inline_values: &mut graph.inline_values,
            inputs: &mut graph.inputs,
            outputs: &mut graph.outputs,
            registry,
            side_effects,
            cache,
        }
    }
}

impl<'a, 'snarl> GraphEditingContext<'a, 'snarl> {
    pub fn as_execution_context(&mut self) -> GraphExecutionContext {
        GraphExecutionContext::new(
            self.snarl,
            self.inline_values,
            self.registry,
            self.side_effects.clone(),
            self.cache,
        )
    }

    /// Ensures that the inline input value of the given pin is present
    pub fn ensure_inline_input(&mut self, pin: InPinId) -> miette::Result<bool> {
        let node = &self.snarl[pin.node];

        if !node.has_inline_values()? {
            return Ok(false);
        }

        match self.inline_values.entry(pin) {
            Entry::Occupied(_) => Ok(true),
            Entry::Vacant(e) => {
                let value = node.default_input_value(self.registry, pin.input)?;
                e.insert(value.into_owned());
                Ok(true)
            }
        }
    }

    /// Returns mutable reference to the input value of the given pin
    pub fn get_inline_input_mut(&mut self, pin: InPinId) -> miette::Result<Option<&mut EValue>> {
        if !self.ensure_inline_input(pin)? {
            return Ok(None);
        }

        Ok(Some(
            self.inline_values
                .get_mut(&pin)
                .expect("input value should be present"),
        ))
    }

    pub fn connect(
        &mut self,
        from: &OutPin,
        to: &InPin,
        commands: &mut SnarlCommands,
    ) -> miette::Result<()> {
        m_try(|| {
            let from_ty = &self.snarl[from.id.node]
                .try_output(self.registry, from.id.output)?
                .ty;

            let to_node = &mut self.snarl[to.id.node];

            to_node.try_connect(self.registry, commands, from, to, from_ty)?;

            Ok(())
        })
        .with_context(|| format!("failed to connect pins: {:?} -> {:?}", from.id, to.id))?;

        commands.execute(self)
    }

    pub fn disconnect(
        &mut self,
        from: &OutPin,
        to: &InPin,
        commands: &mut SnarlCommands,
    ) -> miette::Result<()> {
        self.snarl[to.id.node].try_disconnect(self.registry, commands, from, to)?;

        commands.execute(self)
    }

    pub fn remove_node(
        &mut self,
        node: NodeId,
        commands: &mut SnarlCommands,
    ) -> miette::Result<()> {
        commands.push(SnarlCommand::DeleteNode { node });

        commands
            .execute(self)
            .with_context(|| format!("failed to remove node: {:?}", node))
    }

    pub fn create_node(
        &mut self,
        id: Ustr,
        pos: Pos2,
        _commands: &mut SnarlCommands,
    ) -> miette::Result<NodeId> {
        let id = self.snarl.insert_node(pos, get_snarl_node(&id).unwrap());
        self.inline_values.retain(|in_pin, _| in_pin.node != id);

        Ok(id)
    }

    pub fn create_object_node(
        &mut self,
        object: ETypeId,
        pos: Pos2,
        _commands: &mut SnarlCommands,
    ) -> miette::Result<NodeId> {
        let node: SnarlNode = match self
            .registry
            .get_object(&object)
            .expect("object id should be valid")
        {
            EObjectType::Struct(_) => Box::new(StructNode::new(object)),
            EObjectType::Enum(data) => Box::new(EnumNode::new(data.variant_ids()[0])),
        };

        let id = self.snarl.insert_node(pos, node);
        self.inline_values.retain(|in_pin, _| in_pin.node != id);

        Ok(id)
    }

    pub fn create_list_node(
        &mut self,
        item_ty: EListId,
        pos: Pos2,
        _commands: &mut SnarlCommands,
    ) -> miette::Result<NodeId> {
        let item_ty = self
            .registry
            .get_list(&item_ty)
            .expect("list id should be valid")
            .value_type;
        let node = Box::new(ListNode::of_type(item_ty));
        let id = self.snarl.insert_node(pos, node);
        self.inline_values.retain(|in_pin, _| in_pin.node != id);

        Ok(id)
    }

    pub fn mark_dirty(&mut self, node: NodeId) {
        self.as_execution_context().mark_dirty(node)
    }

    pub fn read_output(&mut self, id: OutPinId) -> miette::Result<EValue> {
        self.as_execution_context().read_output(id)
    }
    pub fn read_input(&mut self, id: InPinId) -> miette::Result<EValue> {
        self.as_execution_context().read_input(id)
    }
}

#[derive(Debug)]
pub struct PartialGraphEditingContext<'a> {
    pub inline_values: &'a mut AHashMap<InPinId, EValue>,
    pub inputs: &'a mut SmallVec<[GraphInput; 1]>,
    pub outputs: &'a mut SmallVec<[GraphOutput; 1]>,
    pub registry: &'a ETypesRegistry,
    side_effects: SideEffectsContext<'a>,
    cache: &'a mut GraphCache,
}

impl<'a> PartialGraphEditingContext<'a> {
    pub fn from_graph(
        graph: &'a mut Graph,
        registry: &'a ETypesRegistry,
        cache: &'a mut GraphCache,
        side_effects: SideEffectsContext<'a>,
    ) -> (Self, &'a mut Snarl<SnarlNode>) {
        (
            PartialGraphEditingContext {
                inline_values: &mut graph.inline_values,
                inputs: &mut graph.inputs,
                cache,
                registry,
                side_effects,
                outputs: &mut graph.outputs,
            },
            &mut graph.snarl,
        )
    }

    pub fn as_full<'b, 'snarl>(
        &'b mut self,
        snarl: &'snarl mut Snarl<SnarlNode>,
    ) -> GraphEditingContext<'b, 'snarl>
    where
        'a: 'b,
    {
        GraphEditingContext {
            snarl,
            inline_values: self.inline_values,
            cache: self.cache,
            registry: self.registry,
            side_effects: self.side_effects.clone(),
            inputs: self.inputs,
            outputs: self.outputs,
        }
    }
}
