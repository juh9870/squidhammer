use crate::graph::execution::GraphExecutionContext;
use crate::graph::node::commands::SnarlCommands;
use crate::graph::node::SnarlNode;
use crate::graph::Graph;
use crate::m_try;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use ahash::AHashMap;
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};
use miette::Context;

#[derive(Debug)]
pub struct PartialGraphExecutionContext<'a> {
    pub inputs: &'a mut AHashMap<InPinId, EValue>,
    pub registry: &'a ETypesRegistry,
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
            },
            ctx.snarl,
        )
    }

    pub fn from_graph(
        graph: &'a mut Graph,
        registry: &'a ETypesRegistry,
    ) -> (Self, &'a mut Snarl<SnarlNode>) {
        (
            PartialGraphExecutionContext {
                inputs: &mut graph.inputs,
                cache: &mut graph.cache,
                registry,
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
        }
    }

    pub fn mark_dirty(&mut self, snarl: &Snarl<SnarlNode>, node: NodeId) {
        self.as_full(snarl).mark_dirty(node)
    }

    pub fn full_eval(&mut self, snarl: &Snarl<SnarlNode>) -> miette::Result<()> {
        self.as_full(snarl).full_eval()
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

        commands.execute(self, snarl, self.registry)
    }

    pub fn disconnect(
        &mut self,
        from: &OutPin,
        to: &InPin,
        snarl: &mut Snarl<SnarlNode>,
        commands: &mut SnarlCommands,
    ) -> miette::Result<()> {
        snarl[to.id.node].try_disconnect(self.registry, commands, from, to)?;

        commands.execute(self, snarl, self.registry)
    }
}
