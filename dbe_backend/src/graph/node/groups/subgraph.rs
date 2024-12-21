use crate::graph::cache::GraphCache;
use crate::graph::execution::GraphExecutionContext;
use crate::graph::node::commands::SnarlCommands;
use crate::graph::node::groups::utils::{
    get_port_input, get_port_output, map_group_inputs, map_group_outputs, sync_fields,
};
use crate::graph::node::ports::{InputData, OutputData};
use crate::graph::node::{
    impl_serde_node, ExecutionExtras, Node, NodeContext, NodeFactory, SnarlNode,
};
use crate::m_try;
use crate::project::project_graph::ProjectGraph;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::NodeId;
use miette::{bail, miette, Context};
use serde::{Deserialize, Serialize};
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubgraphNode {
    graph: Uuid,
    inputs: Vec<Uuid>,
    outputs: Vec<Uuid>,
}

impl SubgraphNode {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_graph<'ctx>(
        &self,
        context: NodeContext<'ctx>,
    ) -> miette::Result<&'ctx ProjectGraph> {
        let graphs = context.graphs.ok_or_else(|| miette!("No graph context"))?;
        graphs
            .graphs
            .get(&self.graph)
            .ok_or_else(|| miette!("No graph context"))
    }
}

impl Node for SubgraphNode {
    impl_serde_node!();

    fn id(&self) -> Ustr {
        SubgraphNodeFactory.id()
    }

    fn title(&self, context: NodeContext) -> String {
        let Ok(graph) = self.get_graph(context) else {
            return "!!unknown graph!!".to_string();
        };
        graph.name.clone()
    }

    fn update_state(&mut self, context: NodeContext, commands: &mut SnarlCommands, id: NodeId) {
        let Ok(graph) = self.get_graph(context) else {
            return;
        };

        sync_fields(commands, graph.inputs(), &mut self.inputs, id);
        sync_fields(commands, graph.outputs(), &mut self.outputs, id);
    }

    fn inputs_count(&self, context: NodeContext) -> usize {
        let Ok(graph) = self.get_graph(context) else {
            return 0;
        };
        graph.inputs().len()
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        let graph = self.get_graph(context)?;
        get_port_input(graph.inputs(), &self.inputs, input)
    }

    fn outputs_count(&self, context: NodeContext) -> usize {
        let Ok(graph) = self.get_graph(context) else {
            return 0;
        };
        graph.outputs().len()
    }

    fn output_unchecked(&self, context: NodeContext, output: usize) -> miette::Result<OutputData> {
        let graph = self.get_graph(context)?;
        get_port_output(graph.outputs(), &self.inputs, output)
    }

    fn execute(
        &self,
        context: NodeContext,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<()> {
        let graph = self.get_graph(context)?;

        if !graph.is_node_group {
            bail!("Graph `{}` ({}) is not a node group", graph.name, graph.id);
        }

        let mut graph_in = Vec::with_capacity(graph.inputs().len());
        let mut graph_out = None;

        m_try(|| {
            map_group_inputs(graph.inputs(), &self.inputs, inputs, &mut graph_in)?;

            // TODO: caching for subgraphs
            let mut cache = GraphCache::default();
            let mut ctx = GraphExecutionContext::from_graph(
                graph.graph(),
                context.registry,
                context.graphs,
                &mut cache,
                variables.side_effects.clone(), // TODO: proper side effects context
                &graph_in,
                &mut graph_out,
            );

            ctx.full_eval(true)?;

            if graph.outputs().is_empty() {
                return Ok(());
            }

            let graph_out =
                graph_out.ok_or_else(|| miette!("Node group did not emit any outputs"))?;

            map_group_outputs(
                context.registry,
                graph.outputs(),
                &self.outputs,
                &graph_out,
                outputs,
            )?;
            Ok(())
        })
        .with_context(|| {
            format!(
                "failed to execute node group `{}` ({})",
                graph.name, graph.id
            )
        })?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SubgraphNodeFactory;

impl NodeFactory for SubgraphNodeFactory {
    fn id(&self) -> Ustr {
        "subgraph".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &[]
    }

    fn create(&self) -> SnarlNode {
        Box::new(SubgraphNode::new())
    }
}
