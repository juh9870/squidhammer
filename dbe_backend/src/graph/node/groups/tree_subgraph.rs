use crate::etype::default::DefaultEValue;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::extras::ExecutionExtras;
use crate::graph::node::groups::utils::sync_fields;
use crate::graph::node::ports::fields::IoDirection;
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::{ExecutionResult, Node, NodeContext, NodeFactory, SnarlNode};
use crate::graph::Graph;
use crate::json_utils::json_serde::JsonSerde;
use crate::json_utils::JsonValue;
use crate::project::docs::{Docs, DocsRef};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::{InPin, NodeId, OutPin, OutPinId};
use emath::pos2;
use miette::bail;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone, Hash)]
pub struct TreeSubgraph {
    tree: tree::TreeGraphData,
    inputs: Vec<Uuid>,
    outputs: Vec<Uuid>,
    connected_inputs: Vec<bool>,
}

impl TreeSubgraph {
    pub fn new(node: SnarlNode) -> Self {
        let mut graph = Graph::default();
        let root = graph.snarl.insert_node(pos2(0.0, 0.0), node);
        Self {
            tree: tree::TreeGraphData::new(graph, root),
            inputs: vec![],
            outputs: vec![],
            connected_inputs: vec![],
        }
    }
}

impl Node for TreeSubgraph {
    fn write_json(&self, registry: &ETypesRegistry) -> miette::Result<JsonValue> {
        self.tree.write_json(registry, ())
    }

    fn parse_json(
        &mut self,
        registry: &ETypesRegistry,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        self.tree.parse_json(registry, (), value)
    }

    fn id(&self) -> Ustr {
        TreeSubgraphFactory.id()
    }

    fn default_input_value(
        &self,
        context: NodeContext,
        input: usize,
    ) -> miette::Result<DefaultEValue> {
        let Some((node, pin)) = self.tree.node_for_input(input) else {
            bail!("input cache not initialized")
        };

        node.default_input_value(context, pin.input)
    }

    fn title(&self, context: NodeContext, docs: &Docs) -> String {
        let title = self.tree.root_node().title(context, docs);
        format!("{} (Nested)", title)
    }

    fn update_state(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        id: NodeId,
    ) -> miette::Result<()> {
        self.tree.update_nodes_state(context)?;

        let changed = self
            .tree
            .sync_tree_state(context, &mut self.connected_inputs, false)?;

        if changed {
            let inputs = self.tree.group_inputs();

            sync_fields(
                commands,
                inputs,
                &mut self.inputs,
                None,
                id,
                IoDirection::Input(0),
            );

            self.outputs.clear();
            self.outputs
                .extend(self.tree.group_inputs().iter().map(|x| x.id));

            for idx in 0..self.outputs.len() {
                commands.push(SnarlCommand::ReconnectOutput {
                    id: OutPinId {
                        node: id,
                        output: idx,
                    },
                })
            }
        }

        Ok(())
    }

    fn has_inline_values(&self, input: usize) -> miette::Result<bool> {
        let Some((node, pin)) = self.tree.node_for_input(input) else {
            bail!("input cache not initialized")
        };

        node.has_inline_values(pin.input)
    }

    fn inputs_count(&self, _context: NodeContext) -> usize {
        self.inputs.len()
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        let Some((node, pin)) = self.tree.node_for_input(input) else {
            bail!("input cache not initialized")
        };

        let mut input = node.input_unchecked(context, pin.input)?;

        if input.custom_docs.is_none() {
            input.custom_docs = Some(DocsRef::NodeInput(node.node.id(), input.name));
        }

        Ok(input)
    }

    fn outputs_count(&self, _context: NodeContext) -> usize {
        self.outputs.len()
    }

    fn output_unchecked(&self, context: NodeContext, output: usize) -> miette::Result<OutputData> {
        let (node, pin) = self.tree.node_for_output(output);

        let mut output = node.output_unchecked(context, pin.output)?;

        if output.custom_docs.is_none() {
            output.custom_docs = Some(DocsRef::NodeInput(node.node.id(), output.name));
        }

        Ok(output)
    }

    fn try_connect(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<bool> {
        if self._default_try_connect(context, commands, from, to, incoming_type)? {
            self.connected_inputs[to.id.input] = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn try_disconnect(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
    ) -> miette::Result<()> {
        self.connected_inputs[to.id.input] = false;
        self._default_try_disconnect(context, commands, from, to)
    }

    fn has_side_effects(&self) -> bool {
        self.tree.all_nodes().any(|n| n.1.has_side_effects())
    }

    fn execute(
        &self,
        context: NodeContext,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        self.tree.execute(context, inputs, outputs, variables)?;
        Ok(ExecutionResult::Done)
    }
}

#[derive(Debug, Clone)]
pub struct TreeSubgraphFactory;

impl NodeFactory for TreeSubgraphFactory {
    fn id(&self) -> Ustr {
        "tree_subgraph".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &[]
    }

    fn create(&self) -> Box<dyn Node> {
        Box::new(TreeSubgraph {
            tree: tree::TreeGraphData::new_invalid(),
            inputs: vec![],
            outputs: vec![],
            connected_inputs: vec![],
        })
    }
}

mod tree {
    use crate::graph::editing::GraphEditingContext;
    use crate::graph::execution::GraphExecutionContext;
    use crate::graph::inputs::{GraphInput, GraphOutput};
    use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
    use crate::graph::node::extras::ExecutionExtras;
    use crate::graph::node::groups::input::GroupInputNode;
    use crate::graph::node::groups::output::GroupOutputNode;
    use crate::graph::node::{NodeContext, SnarlNode};
    use crate::graph::Graph;
    use crate::json_utils::json_serde::JsonSerde;
    use crate::json_utils::JsonValue;
    use crate::project::side_effects::SideEffectsContext;
    use crate::registry::ETypesRegistry;
    use crate::value::EValue;
    use collection_traits::Iterable;
    use egui_snarl::{InPinId, NodeId, OutPinId};
    use emath::pos2;
    use itertools::Itertools;
    use miette::{bail, miette, Context, IntoDiagnostic};
    use serde_json::json;
    use smallvec::smallvec;
    use std::collections::BTreeMap;
    use std::hash::{Hash, Hasher};
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    pub struct TreeGraphData {
        graph: Graph,
        root: NodeId,
        tree: BTreeMap<NodeId, Vec<Option<NodeId>>>,
        input_ids: BTreeMap<InPinId, Uuid>,
        // cached data
        inputs: Option<Vec<InPinId>>,
        group_input_node: Option<NodeId>,
        group_output_node: Option<NodeId>,
    }

    impl TreeGraphData {
        pub fn new(graph: Graph, root: NodeId) -> Self {
            Self {
                graph,
                tree: [(root, vec![])].into_iter().collect(),
                root,
                input_ids: Default::default(),
                inputs: None,
                group_input_node: None,
                group_output_node: None,
            }
        }

        pub fn new_invalid() -> Self {
            Self {
                graph: Default::default(),
                root: NodeId(usize::MAX),
                tree: Default::default(),
                input_ids: Default::default(),
                inputs: None,
                group_input_node: None,
                group_output_node: None,
            }
        }

        pub fn group_inputs(&self) -> &[GraphInput] {
            self.graph.inputs.as_slice()
        }

        // pub fn group_outputs(&self) -> &[GraphOutput] {
        //     self.graph.outputs.as_slice()
        // }

        pub fn node_for_input(&self, input: usize) -> Option<(&SnarlNode, InPinId)> {
            let inputs = self.inputs.as_ref()?;

            let pin = inputs.get(input)?;

            let node = self.graph.snarl.get_node(pin.node)?;

            Some((node, *pin))
        }

        pub fn node_for_output(&self, output: usize) -> (&SnarlNode, OutPinId) {
            (
                self.graph
                    .snarl
                    .get_node(self.root)
                    .expect("Root node should exist"),
                OutPinId {
                    node: self.root,
                    output,
                },
            )
        }

        pub fn root_node(&self) -> &SnarlNode {
            self.graph
                .snarl
                .get_node(self.root)
                .expect("Root node should exist")
        }

        pub fn all_nodes(&self) -> impl Iterator<Item = (NodeId, &SnarlNode)> {
            self.graph.snarl.node_ids()
        }

        pub fn update_nodes_state(&mut self, context: NodeContext) -> miette::Result<()> {
            let mut commands = SnarlCommands::new();

            for (id, node) in self.graph.snarl.nodes_ids_mut() {
                node.update_state(context, &mut commands, id)?;
            }

            let mut _outputs = None;
            let mut ctx = GraphEditingContext::from_graph_and_context(
                &mut self.graph,
                context,
                SideEffectsContext::Unavailable,
                true,
                &[],
                &mut _outputs,
            );
            commands
                .execute(&mut ctx)
                .context("failed to execute state update commands")?;

            Ok(())
        }

        pub fn execute(
            &self,
            context: NodeContext,
            inputs: &[EValue],
            outputs: &mut Vec<EValue>,
            variables: &mut ExecutionExtras,
        ) -> miette::Result<()> {
            outputs.clear();
            let mut sub_outputs = Some(std::mem::take(outputs));

            let side_effects_available = variables.side_effects.is_available();
            let mut ctx = GraphExecutionContext::from_graph_and_context(
                &self.graph,
                context,
                variables.side_effects.with_subgraph(Uuid::default()),
                true,
                inputs,
                &mut sub_outputs,
            );

            ctx.full_eval(side_effects_available)
                .context("failed to execute tree_subgraph")?;

            drop(ctx);

            *outputs = sub_outputs.ok_or_else(|| miette!("Node group did not emit any outputs"))?;

            Ok(())
        }
    }

    impl TreeGraphData {
        pub fn sync_tree_state(
            &mut self,
            context: NodeContext,
            connected_inputs: &mut Vec<bool>,
            full_check: bool,
        ) -> miette::Result<bool> {
            let mut commands = SnarlCommands::new();
            self.calculate_inputs(context);
            let (group_input_node, group_output_node) = self.get_group_io_nodes();

            let mut body_correct = !full_check || self.check_body_correctness(context);
            let mut inputs_correct =
                body_correct && self.check_inputs_correctness(context, connected_inputs);
            let mut outputs_correct = body_correct && self.check_output_correctness(context);

            if !full_check && (!inputs_correct || !outputs_correct) {
                body_correct = self.check_body_correctness(context);
                if !body_correct {
                    inputs_correct = false;
                    outputs_correct = false;
                }
            }

            if body_correct && inputs_correct && outputs_correct {
                return Ok(false);
            }

            if !body_correct {
                for (out_pin, in_pin) in self.graph.snarl.wires() {
                    commands.push(SnarlCommand::Disconnect {
                        from: out_pin,
                        to: in_pin,
                    });
                }

                for (id, connections) in &self.tree {
                    for (idx, in_node) in connections
                        .iter()
                        .enumerate()
                        .filter_map(|x| x.1.map(|y| (x.0, y)))
                    {
                        commands.push(SnarlCommand::Connect {
                            from: OutPinId {
                                node: in_node,
                                output: 0,
                            },
                            to: InPinId {
                                node: *id,
                                input: idx,
                            },
                        });
                    }
                }
            }

            // inputs
            if !inputs_correct {
                // no need to drop again if whole body was reconstructed
                if body_correct {
                    commands.push(SnarlCommand::DropNodeOutputs {
                        from: group_input_node,
                    });
                }

                connected_inputs.resize_with(self.inputs.as_ref().unwrap().len(), || false);

                let mut ids = vec![];
                for ((idx, pin), is_connected) in self
                    .inputs
                    .as_ref()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .zip_eq(connected_inputs.iter().copied())
                {
                    let id = *self.input_ids.entry(*pin).or_insert_with(Uuid::new_v4);
                    let pin_node = &self.graph.snarl[pin.node].node;
                    let in_pin = pin_node.try_input(context, pin.input).with_context(|| {
                        format!(
                            "failed to read input of node {}({:?})",
                            pin_node.id(),
                            pin.node
                        )
                    })?;
                    ids.push((id, in_pin));
                    if is_connected {
                        commands.push(SnarlCommand::Connect {
                            from: OutPinId {
                                node: group_input_node,
                                output: idx,
                            },
                            to: *pin,
                        })
                    }
                }

                // Only keep live IDs
                self.input_ids
                    .retain(|id, _| self.inputs.as_ref().unwrap().iter().any(|x| x == id));

                self.graph.inputs = ids
                    .iter()
                    .map(|(id, pin)| GraphInput {
                        id: *id,
                        ty: Some(pin.ty.ty()),
                        name: pin.name.to_string(),
                    })
                    .collect();

                let in_node = self.graph.snarl[group_input_node]
                    .downcast_mut::<GroupInputNode>()
                    .expect("Group input node should have correct type");

                in_node.ids = ids.iter().map(|x| x.0).collect();
            }

            if !outputs_correct {
                // no need to drop again if whole body was reconstructed
                if body_correct {
                    commands.push(SnarlCommand::DropNodeInputs {
                        to: group_output_node,
                    });
                    commands.push(SnarlCommand::DropNodeOutputs { from: self.root });
                }

                let root_node = &self.graph.snarl[self.root].node;

                let count = root_node.outputs_count(context);
                self.graph.outputs = smallvec![];

                for i in 0..count {
                    let port = root_node.try_output(context, i).with_context(|| {
                        format!(
                            "failed to read output of root node {}({:?})",
                            root_node.id(),
                            self.root
                        )
                    })?;

                    let id = Uuid::new_v4();
                    self.graph.outputs.push(GraphOutput {
                        id,
                        ty: Some(port.ty.ty()),
                        name: port.name.to_string(),
                    });

                    commands.push(SnarlCommand::Connect {
                        from: OutPinId {
                            node: self.root,
                            output: i,
                        },
                        to: InPinId {
                            node: group_output_node,
                            input: i,
                        },
                    });
                }

                self.graph.snarl[group_output_node]
                    .node
                    .downcast_mut::<GroupOutputNode>()
                    .expect("Group output node should have correct type")
                    .ids = self.graph.outputs.iter().map(|x| x.id).collect();
            }

            let mut _outputs = None;
            let mut ctx = GraphEditingContext::from_graph_and_context(
                &mut self.graph,
                context,
                SideEffectsContext::Unavailable,
                true,
                &[],
                &mut _outputs,
            );
            commands
                .execute(&mut ctx)
                .context("failed to execute reconstruction commands")?;

            drop(ctx);

            if !self.check_body_correctness(context)
                || !self.check_inputs_correctness(context, connected_inputs)
                || !self.check_output_correctness(context)
            {
                bail!("tree graph state is still invalid after reconstruction");
            }

            Ok(true)
        }

        fn check_inputs_correctness(
            &mut self,
            context: NodeContext,
            connected_inputs: &mut Vec<bool>,
        ) -> bool {
            let (input, _) = self.get_group_io_nodes();
            self.calculate_inputs(context);
            let inputs = self.inputs.as_ref().unwrap();
            let group_input_connections: BTreeMap<_, _> = self
                .graph
                .snarl
                .wires()
                .filter(|w| w.0.node == input)
                .map(|(i, o)| (i.output, o))
                .collect();

            if inputs.len() != self.graph.inputs().len() || inputs.len() != connected_inputs.len() {
                return false;
            }

            for (((idx, pin), is_connected), graph_input) in inputs
                .iter()
                .enumerate()
                .zip_eq(connected_inputs.iter().copied())
                .zip_eq(self.graph.inputs.iter_mut())
            {
                let Some(uuid) = self.input_ids.get(pin) else {
                    return false;
                };
                if graph_input.id != *uuid {
                    graph_input.id = *uuid;
                    return false;
                }
                if is_connected {
                    let Some(connection) = group_input_connections.get(&idx) else {
                        return false;
                    };

                    if connection != pin {
                        return false;
                    }
                } else if group_input_connections.contains_key(&idx) {
                    return false;
                }
            }

            true
        }

        fn check_output_correctness(&mut self, _context: NodeContext) -> bool {
            let (_, output) = self.get_group_io_nodes();
            let mut wires = 0;
            for (out_pin, in_pin) in self.graph.snarl.wires() {
                if out_pin.node != self.root && in_pin.node != output {
                    continue;
                }
                if out_pin.node != self.root || in_pin.node != output {
                    return false;
                }
                if out_pin.output != in_pin.input {
                    return false;
                }
                wires += 1;
            }
            self.graph.outputs.len() == wires
        }

        fn check_body_correctness(&mut self, _context: NodeContext) -> bool {
            let (input, output) = self.get_group_io_nodes();

            for (out_pin, in_pin) in self.graph.snarl.wires() {
                if in_pin.node == output || out_pin.node == input {
                    continue;
                }
                if out_pin.output != 0 {
                    return false;
                }

                let Some(target) = self.tree.get(&in_pin.node) else {
                    return false;
                };

                let Some(node) = target.get(in_pin.input).and_then(|x| x.as_ref()) else {
                    return false;
                };

                if node != &out_pin.node {
                    return false;
                }
            }

            true
        }

        fn get_group_io_nodes(&mut self) -> (NodeId, NodeId) {
            let snarl = &mut self.graph.snarl;

            let group_input_node = if let Some(node) = self.group_input_node {
                node
            } else {
                let group_input_node = if let Some(node) = snarl
                    .node_ids()
                    .find(|(_, node)| node.downcast_ref::<GroupInputNode>().is_some())
                    .map(|(id, _)| id)
                {
                    node
                } else {
                    let input = GroupInputNode::new();
                    snarl.insert_node(pos2(0.0, 0.0), SnarlNode::new(Box::new(input)))
                };

                self.group_input_node = Some(group_input_node);
                group_input_node
            };

            let group_output_node = if let Some(node) = self.group_output_node {
                node
            } else {
                let group_output_node = if let Some(node) = snarl
                    .node_ids()
                    .find(|(_, node)| node.downcast_ref::<GroupOutputNode>().is_some())
                    .map(|(id, _)| id)
                {
                    node
                } else {
                    let output = GroupOutputNode::new();
                    snarl.insert_node(pos2(0.0, 0.0), SnarlNode::new(Box::new(output)))
                };

                self.group_output_node = Some(group_output_node);
                group_output_node
            };

            (group_input_node, group_output_node)
        }

        /// Calculates "leaf" inputs, that should be connected to the group input
        fn calculate_inputs(&mut self, context: NodeContext) -> &[InPinId] {
            if self.inputs.is_none() {
                enum Item {
                    Input(InPinId),
                    Node(NodeId),
                }

                let mut inputs = vec![Item::Node(self.root)];
                let mut tmp_hold = vec![];

                let mut i: usize = 0;
                while i < inputs.len() {
                    match &inputs[i] {
                        Item::Input(_) => {
                            i += 1;
                        }
                        Item::Node(node) => {
                            let tree_node = self.tree.get_mut(node).unwrap();
                            let snarl_node = &self.graph.snarl[*node];
                            let count = snarl_node.inputs_count(context);
                            tree_node.resize_with(count, Default::default);

                            for (idx, input) in tree_node.iter().enumerate() {
                                if let Some(input) = input {
                                    tmp_hold.push(Item::Node(*input));
                                } else {
                                    tmp_hold.push(Item::Input(InPinId {
                                        node: *node,
                                        input: idx,
                                    }));
                                }
                            }

                            inputs.splice(i..=i, tmp_hold.drain(..));

                            i += 1;
                        }
                    }
                }

                self.inputs = Some(
                    inputs
                        .into_iter()
                        .map(|x| match x {
                            Item::Input(input) => input,
                            _ => unreachable!(),
                        })
                        .collect(),
                );
            };

            self.inputs.as_ref().unwrap()
        }
    }

    impl JsonSerde for TreeGraphData {
        type State<'a> = ();

        fn write_json(
            &self,
            registry: &ETypesRegistry,
            _: Self::State<'_>,
        ) -> miette::Result<JsonValue> {
            let graph = self.graph.write_json(registry)?;
            Ok(json!({
                "graph": graph,
                "root": self.root,
                "tree": self.tree,
                "input_ids": self.input_ids,
            }))
        }

        fn parse_json(
            &mut self,
            registry: &ETypesRegistry,
            _: Self::State<'_>,
            value: &mut JsonValue,
        ) -> miette::Result<()> {
            let JsonValue::Object(mut obj) = value.take() else {
                bail!("expected object");
            };
            self.graph = Graph::parse_json(
                registry,
                obj.get_mut("graph")
                    .ok_or_else(|| miette!("missing `graph` field"))?,
            )?;
            self.root = serde_json::from_value(
                obj.remove("root")
                    .ok_or_else(|| miette!("missing `root` field"))?,
            )
            .into_diagnostic()?;
            self.tree = serde_json::from_value(
                obj.remove("tree")
                    .ok_or_else(|| miette!("missing `tree` field"))?,
            )
            .into_diagnostic()?;
            self.input_ids = serde_json::from_value(
                obj.remove("input_ids")
                    .ok_or_else(|| miette!("missing `input_ids` field"))?,
            )
            .into_diagnostic()?;

            Ok(())
        }
    }

    impl Hash for TreeGraphData {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.graph.hash(state);
            self.root.hash(state);
            self.tree.hash(state);
            self.input_ids.hash(state);
        }
    }
}
