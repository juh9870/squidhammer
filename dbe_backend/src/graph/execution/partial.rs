use crate::graph::cache::GraphCache;
use crate::graph::execution::GraphExecutionContext;
use crate::graph::node::SnarlNode;
use crate::graph::Graph;
use crate::project::side_effects::SideEffectsContext;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use ahash::AHashMap;
use egui_snarl::{InPinId, NodeId, OutPinId, Snarl};

#[derive(Debug)]
pub struct PartialGraphExecutionContext<'a> {
    pub inline_values: &'a AHashMap<InPinId, EValue>,
    pub registry: &'a ETypesRegistry,
    pub side_effects: SideEffectsContext<'a>,
    cache: &'a mut GraphCache,
}

impl<'a> PartialGraphExecutionContext<'a> {
    pub fn from_context<'b, 'snarl>(
        ctx: &'a mut GraphExecutionContext<'b, 'snarl>,
    ) -> (Self, &'snarl Snarl<SnarlNode>) {
        (
            PartialGraphExecutionContext {
                inline_values: ctx.inline_values,
                cache: ctx.cache,
                registry: ctx.registry,
                side_effects: ctx.side_effects.clone(),
            },
            ctx.snarl,
        )
    }

    pub fn from_graph(
        graph: &'a Graph,
        registry: &'a ETypesRegistry,
        cache: &'a mut GraphCache,
        side_effects: SideEffectsContext<'a>,
    ) -> (Self, &'a Snarl<SnarlNode>) {
        (
            PartialGraphExecutionContext {
                inline_values: &graph.inline_values,
                cache,
                registry,
                side_effects,
            },
            &graph.snarl,
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
            inline_values: self.inline_values,
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

    pub fn read_input(&mut self, snarl: &Snarl<SnarlNode>, id: InPinId) -> miette::Result<EValue> {
        self.as_full(snarl).read_input(id)
    }
}
