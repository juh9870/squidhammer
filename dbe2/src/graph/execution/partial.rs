use crate::graph::execution::GraphExecutionContext;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::enum_node::EnumNode;
use crate::graph::node::struct_node::StructNode;
use crate::graph::node::{get_snarl_node, SnarlNode};
use crate::graph::Graph;
use crate::m_try;
use crate::project::side_effects::SideEffectsContext;
use crate::registry::{EObjectType, ETypesRegistry};
use crate::value::id::ETypeId;
use crate::value::EValue;
use ahash::AHashMap;
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};
use emath::Pos2;
use miette::Context;
use ustr::Ustr;

#[derive(Debug)]
pub struct PartialGraphExecutionContext<'a> {
    pub inputs: &'a mut AHashMap<InPinId, EValue>,
    pub registry: &'a ETypesRegistry,
    pub side_effects: SideEffectsContext<'a>,
    cache: &'a mut AHashMap<NodeId, Vec<EValue>>,
}

impl<'a> PartialGraphExecutionContext<'a> {
    pub fn from_context<'b, 'snarl>(
        ctx: &'a mut GraphExecutionContext<'b, 'snarl>,
    ) -> (Self, &'snarl Snarl<SnarlNode>) {
        (
            PartialGraphExecutionContext {
                inputs: ctx.inputs,
                cache: ctx.cache,
                registry: ctx.registry,
                side_effects: ctx.side_effects.clone(),
            },
            ctx.snarl,
        )
    }

    pub fn from_graph(
        graph: &'a mut Graph,
        registry: &'a ETypesRegistry,
        side_effects: SideEffectsContext<'a>,
    ) -> (Self, &'a mut Snarl<SnarlNode>) {
        (
            PartialGraphExecutionContext {
                inputs: &mut graph.inputs,
                cache: &mut graph.cache,
                registry,
                side_effects,
            },
            &mut graph.snarl,
        )
    }

    pub fn as_full<'b, 'snarl>(
        &'b mut self,
        snarl: &'snarl Snarl<SnarlNode>,
    ) -> GraphExecutionContext<'b, 'snarl>
    where
        'a: 'b,
    {
        GraphExecutionContext {
            snarl,
            inputs: self.inputs,
            cache: self.cache,
            registry: self.registry,
            side_effects: self.side_effects.clone(),
        }
    }

    pub fn mark_dirty(&mut self, snarl: &Snarl<SnarlNode>, node: NodeId) {
        self.as_full(snarl).mark_dirty(node)
    }

    pub fn full_eval(
        &mut self,
        snarl: &Snarl<SnarlNode>,
        side_effects: bool,
    ) -> miette::Result<()> {
        self.as_full(snarl).full_eval(side_effects)
    }

    pub fn read_output(
        &mut self,
        snarl: &Snarl<SnarlNode>,
        id: OutPinId,
    ) -> miette::Result<EValue> {
        self.as_full(snarl).read_output(id)
    }

    /// Ensures that the inline input value of the given pin is present
    pub fn ensure_inline_input(
        &mut self,
        snarl: &Snarl<SnarlNode>,
        pin: InPinId,
    ) -> miette::Result<()> {
        let node = &snarl[pin.node];

        self.as_full(snarl).inline_input_value(pin, node)?;

        Ok(())
    }

    /// Returns mutable reference to the input value of the given pin
    pub fn get_inline_input_mut(
        &mut self,
        snarl: &Snarl<SnarlNode>,
        pin: InPinId,
    ) -> miette::Result<&mut EValue> {
        self.ensure_inline_input(snarl, pin)?;

        Ok(self
            .inputs
            .get_mut(&pin)
            .expect("input value should be present"))
    }

    pub fn read_input(&mut self, snarl: &Snarl<SnarlNode>, id: InPinId) -> miette::Result<EValue> {
        self.as_full(snarl).read_input(id)
    }

    pub fn connect(
        &mut self,
        from: &OutPin,
        to: &InPin,
        snarl: &mut Snarl<SnarlNode>,
        commands: &mut SnarlCommands,
    ) -> miette::Result<()> {
        m_try(|| {
            let from_ty = snarl[from.id.node]
                .try_output(self.registry, from.id.output)?
                .ty;

            let to_node = &mut snarl[to.id.node];

            to_node.try_connect(self.registry, commands, from, to, from_ty)?;

            Ok(())
        })
        .with_context(|| format!("failed to connect pins: {:?} -> {:?}", from.id, to.id))?;

        commands.execute(self, snarl)
    }

    pub fn disconnect(
        &mut self,
        from: &OutPin,
        to: &InPin,
        snarl: &mut Snarl<SnarlNode>,
        commands: &mut SnarlCommands,
    ) -> miette::Result<()> {
        snarl[to.id.node].try_disconnect(self.registry, commands, from, to)?;

        commands.execute(self, snarl)
    }

    pub fn remove_node(
        &mut self,
        node: NodeId,
        snarl: &mut Snarl<SnarlNode>,
        commands: &mut SnarlCommands,
    ) -> miette::Result<()> {
        commands.push(SnarlCommand::DeleteNode { node });

        commands
            .execute(self, snarl)
            .with_context(|| format!("failed to remove node: {:?}", node))
    }

    pub fn create_node(
        &mut self,
        id: Ustr,
        pos: Pos2,
        snarl: &mut Snarl<SnarlNode>,
        _commands: &mut SnarlCommands,
    ) -> miette::Result<NodeId> {
        let id = snarl.insert_node(pos, get_snarl_node(&id).unwrap());
        self.inputs.retain(|in_pin, _| in_pin.node != id);

        Ok(id)
    }

    pub fn create_object_node(
        &mut self,
        object: ETypeId,
        pos: Pos2,
        snarl: &mut Snarl<SnarlNode>,
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

        let id = snarl.insert_node(pos, node);
        self.inputs.retain(|in_pin, _| in_pin.node != id);

        Ok(id)
    }
}
