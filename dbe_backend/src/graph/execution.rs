use crate::graph::cache::GraphCache;
use crate::graph::node::ports::NodePortType;
use crate::graph::node::SnarlNode;
use crate::graph::Graph;
use crate::m_try;
use crate::project::side_effects::SideEffectsContext;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use ahash::AHashMap;
use egui_snarl::{InPinId, NodeId, OutPinId, Snarl};
use miette::{bail, miette, Context};
use smallvec::SmallVec;

#[derive(Debug)]
pub struct GraphExecutionContext<'a, 'snarl> {
    pub snarl: &'snarl Snarl<SnarlNode>,
    pub inline_values: &'a AHashMap<InPinId, EValue>,
    pub registry: &'a ETypesRegistry,
    pub side_effects: SideEffectsContext<'a>,
    cache: &'a mut GraphCache,
}

impl<'a> GraphExecutionContext<'a, 'a> {
    pub fn from_graph(
        graph: &'a Graph,
        registry: &'a ETypesRegistry,
        cache: &'a mut GraphCache,
        side_effects: SideEffectsContext<'a>,
    ) -> Self {
        GraphExecutionContext {
            snarl: &graph.snarl,
            inline_values: &graph.inline_values,
            cache,
            registry,
            side_effects,
        }
    }

    pub(super) fn new(
        snarl: &'a Snarl<SnarlNode>,
        inline_values: &'a AHashMap<InPinId, EValue>,
        registry: &'a ETypesRegistry,
        side_effects: SideEffectsContext<'a>,
        cache: &'a mut GraphCache,
    ) -> Self {
        Self {
            snarl,
            inline_values,
            registry,
            side_effects,
            cache,
        }
    }
}

impl<'a, 'snarl> GraphExecutionContext<'a, 'snarl> {
    /// Marks the node and all downstream nodes as dirty
    pub fn mark_dirty(&mut self, node: NodeId) {
        self.mark_dirty_inner(node, &mut SmallVec::new());
    }

    pub fn full_eval(&mut self, side_effects: bool) -> miette::Result<()> {
        self.cache.clear();
        for (id, has_side_effects) in self
            .snarl
            .node_ids()
            .map(|(id, node)| (id, node.has_side_effects()))
        {
            if !has_side_effects {
                continue;
            }
            if self.cache.contains_key(&id) {
                continue;
            }

            let mut stack = Vec::new();

            self.eval_node_inner(&mut stack, id, side_effects)?
        }

        Ok(())
    }

    pub fn read_output(&mut self, id: OutPinId) -> miette::Result<EValue> {
        let mut stack = Vec::new();
        self.read_node_output_inner(&mut stack, id, false)
    }

    pub fn read_input(&mut self, id: InPinId) -> miette::Result<EValue> {
        let node = self
            .snarl
            .get_node(id.node)
            .ok_or_else(|| miette!("Node {:?} not found", id.node))?;
        let mut stack = Vec::new();
        self.read_node_input_inner(&mut stack, id, node, false)
    }
}

impl<'a, 'snarl> GraphExecutionContext<'a, 'snarl> {
    pub fn mark_dirty_inner(&mut self, node: NodeId, marked: &mut SmallVec<[NodeId; 4]>) {
        if marked.contains(&node) {
            return;
        }
        marked.push(node);

        if self.cache.remove(&node).is_none() {
            // Node was not cached, so it's already dirty
            return;
        }

        for (out_pin, in_pin) in self.snarl.wires() {
            if out_pin.node == node {
                self.mark_dirty_inner(in_pin.node, marked);
            }
        }
    }

    fn read_node_output_inner(
        &mut self,
        stack: &mut Vec<NodeId>,
        pin: OutPinId,
        side_effects: bool,
    ) -> miette::Result<EValue> {
        // trace!("Reading output #{} of node {:?}", pin.output, pin.node);
        m_try(|| {
            if let Some(node) = self.cache.get(&pin.node) {
                return Ok(node
                    .get(pin.output)
                    .ok_or_else(|| miette!("Node doesn't have output #{}", pin.output))?
                    .clone());
            }

            self.eval_node_inner(stack, pin.node, side_effects)?;

            let node = self.cache.get(&pin.node).ok_or_else(|| {
                miette!("!!INTERNAL ERROR!! Node was not cached after evaluation")
            })?;

            Ok(node
                .get(pin.output)
                .ok_or_else(|| miette!("Node doesn't have output #{}", pin.output))?
                .clone())
        })
        .with_context(|| {
            format!(
                "failed to read output #{} of node {:?}",
                pin.output, pin.node
            )
        })
    }

    fn inline_input_value(
        &mut self,
        pin: InPinId,
        node: &SnarlNode,
    ) -> miette::Result<Option<&EValue>> {
        if !node.has_inline_values()? {
            return Ok(None);
        }
        Ok(self.inline_values.get(&pin))
    }

    fn read_node_input_inner(
        &mut self,
        stack: &mut Vec<NodeId>,
        id: InPinId,
        node: &SnarlNode,
        side_effects: bool,
    ) -> miette::Result<EValue> {
        let in_info = node.try_input(self.registry, id.input)?;
        // trace!("Reading input #{} of node {:?}", id.input, id.node);
        m_try(|| {
            // TODO: check for valid types
            let slot = self.snarl.in_pin(id);
            let value = if slot.remotes.is_empty() {
                match self.inline_input_value(slot.id, node)? {
                    None => node
                        .default_input_value(self.registry, id.input)?
                        .into_owned(),
                    Some(val) => val.clone(),
                }
            } else if slot.remotes.len() == 1 {
                let remote = slot.remotes[0];
                let output = self.read_node_output_inner(stack, remote, side_effects)?;
                if output.ty() != in_info.ty.ty() {
                    let remote_info =
                        self.snarl[remote.node].try_output(self.registry, remote.output)?;

                    NodePortType::convert_value(
                        self.registry,
                        &remote_info.ty,
                        &in_info.ty,
                        output,
                    )?
                } else {
                    output
                }
            } else {
                // TODO: allow multi-connect for inputs
                bail!(
                    "Node {:?} input #{} is connected to multiple outputs",
                    id,
                    id.input
                );
            };

            Ok(value)
        })
        .with_context(|| format!("failed to read input #{} of node {:?}", id.input, id.node))
    }

    fn eval_node_inner(
        &mut self,
        stack: &mut Vec<NodeId>,
        id: NodeId,
        side_effects: bool,
    ) -> miette::Result<()> {
        // trace!("Evaluating node {:?}", id);
        m_try(|| {
            if stack.contains(&id) {
                bail!("Cyclic dependency detected");
            }
            stack.push(id);

            let node = self
                .snarl
                .get_node(id)
                .ok_or_else(|| miette!("Node {:?} not found", id))?;

            let inputs_count = node.inputs_count(self.registry);
            let mut input_values = Vec::<EValue>::with_capacity(inputs_count);

            for i in 0..inputs_count {
                input_values.push(self.read_node_input_inner(
                    stack,
                    InPinId { node: id, input: i },
                    node,
                    side_effects,
                )?);
            }

            let outputs_count = node.outputs_count(self.registry);
            let mut outputs = Vec::with_capacity(outputs_count);
            if side_effects && node.has_side_effects() {
                let side_effects = self.side_effects.with_node(id);
                node.execute_side_effects(
                    self.registry,
                    &input_values,
                    &mut outputs,
                    side_effects,
                )?;
            }
            node.execute(self.registry, &input_values, &mut outputs)?;

            // TODO: check for validity of returned values types
            self.cache.insert(id, outputs);

            Ok(())
        })
        .with_context(|| format!("failed to evaluate node {:?}", id))
    }
}

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
}
