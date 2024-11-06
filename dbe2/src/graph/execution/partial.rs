use crate::graph::execution::GraphExecutionContext;
use crate::graph::node::SnarlNode;
use crate::graph::Graph;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use ahash::AHashMap;
use egui_snarl::{InPinId, NodeId, OutPinId, Snarl};

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

    pub fn read_input(&mut self, snarl: &Snarl<SnarlNode>, id: InPinId) -> miette::Result<EValue> {
        self.as_full(snarl).read_input(id)
    }
}
