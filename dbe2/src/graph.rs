use crate::graph::execution::partial::PartialGraphExecutionContext;
use crate::graph::node::commands::SnarlCommands;
use crate::graph::node::{get_snarl_node, SnarlNode};
use crate::json_utils::JsonValue;
use crate::m_try;
use crate::project::side_effects::{SideEffects, SideEffectsContext};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use ahash::AHashMap;
use egui_snarl::{InPinId, NodeId, OutPinId, Snarl};
use emath::Pos2;
use miette::{miette, Context, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use ustr::Ustr;

pub mod execution;
pub mod node;

#[derive(Debug, Default)]
pub struct Graph {
    snarl: Snarl<SnarlNode>,
    inputs: AHashMap<InPinId, EValue>,
    cache: AHashMap<NodeId, Vec<EValue>>,
}
impl Graph {
    pub fn parse_json(registry: &ETypesRegistry, value: &mut JsonValue) -> miette::Result<Self> {
        let mut snarl = Snarl::<SnarlNode>::new();
        let packed: PackedGraph = PackedGraph::deserialize(value.take()).into_diagnostic()?;
        let mut mapping = AHashMap::with_capacity(packed.nodes.len());

        m_try(|| {
            for (serialized_id, mut node) in packed.nodes {
                let mut created_node = get_snarl_node(&node.id)
                    .ok_or_else(|| miette!("node type {} not found", node.id))?;

                created_node.parse_json(registry, &mut node.data)?;

                let node_id = if node.open {
                    snarl.insert_node(node.pos, created_node)
                } else {
                    snarl.insert_node_collapsed(node.pos, created_node)
                };
                mapping.insert(serialized_id, node_id);
            }
            Ok(())
        })
        .context("failed to create nodes")?;

        let inputs = AHashMap::with_capacity(packed.inputs.len());

        let mut graph = Self {
            snarl,
            inputs,
            cache: Default::default(),
        };

        m_try(|| {
            let mut side_effects = SideEffects::default();
            let (mut ctx, snarl) = PartialGraphExecutionContext::from_graph(
                &mut graph,
                registry,
                SideEffectsContext::new(&mut side_effects, "".into()),
            );
            let commands = &mut SnarlCommands::new();

            let mut to_connect = Vec::with_capacity(packed.edges.len());

            for (mut out_pin, mut in_pin) in packed.edges {
                out_pin.node = *mapping
                    .get(&out_pin.node)
                    .ok_or_else(|| miette!("node {:?} not found", out_pin.node))?;
                in_pin.node = *mapping
                    .get(&in_pin.node)
                    .ok_or_else(|| miette!("node {:?} not found", in_pin.node))?;
                // snarl.connect(out_pin, in_pin);
                let out_pin = snarl.out_pin(out_pin);
                let in_pin = snarl.in_pin(in_pin);
                to_connect.push((out_pin, in_pin));
            }

            to_connect.sort_by_key(|(out_pin, in_pin)| {
                (
                    out_pin.id.node,
                    out_pin.id.output,
                    in_pin.id.node,
                    in_pin.id.input,
                )
            });

            for (out_pin, in_pin) in to_connect {
                ctx.connect(&out_pin, &in_pin, snarl, commands)?;
            }

            commands
                .execute(&mut ctx, snarl)
                .with_context(|| "failed sto execute commands")?;

            if !side_effects.is_empty() {
                panic!("Side effects are not supported during deserialization");
            }

            Ok(())
        })
        .context("failed to connect pins")?;

        m_try(|| {
            for (mut in_pin, mut value) in packed.inputs {
                in_pin.node = *mapping
                    .get(&in_pin.node)
                    .ok_or_else(|| miette!("node {:?} not found", in_pin.node))?;

                let node = graph
                    .snarl
                    .get_node(in_pin.node)
                    .expect("Mappings should be correct");

                let input_type = node
                    .try_input(registry, in_pin.input)
                    .with_context(|| format!("failed to get inputs for node {:?}", in_pin.node))?;
                let value = input_type
                    .ty
                    .ty()
                    .parse_json(registry, &mut value, false)
                    .with_context(|| {
                        format!(
                            "failed to parse input value #{} for node {:?}",
                            in_pin.input, in_pin.node
                        )
                    })?;
                graph.inputs.insert(in_pin, value);
            }
            Ok(())
        })
        .context("failed to populate inputs")?;

        Ok(graph)
    }

    pub fn write_json(&self, registry: &ETypesRegistry) -> miette::Result<JsonValue> {
        let mut packed = PackedGraph {
            nodes: Default::default(),
            edges: Default::default(),
            inputs: Default::default(),
        };

        let mut inputs = AHashMap::new();

        for (pin, value) in &self.inputs {
            let value_json = value.write_json(registry).with_context(|| {
                format!(
                    "failed to serialize input value #{} for {:?}",
                    pin.input, pin.node
                )
            })?;
            inputs.insert(*pin, value_json);
        }

        for (id, node) in self.snarl.node_ids() {
            let info = self.snarl.get_node_info(id).expect("Node should exist");
            let packed_node = PackedNode {
                id: node.id(),
                data: node
                    .write_json(registry)
                    .with_context(|| format!("failed to serialize node {:?}", id))?,
                pos: info.pos,
                open: info.open,
            };
            packed.nodes.push((id, packed_node));
        }

        for (out_pin, in_pin) in self.snarl.wires() {
            // do not serialize inputs with connections
            inputs.remove(&in_pin);
            packed.edges.push((out_pin, in_pin));
        }

        packed.inputs = inputs.into_iter().collect();
        packed.inputs.sort_by_key(|(in_pin, _)| *in_pin);
        packed.nodes.sort_by_key(|(id, _)| *id);
        packed
            .edges
            .sort_by_key(|(out_pin, in_pin)| (*out_pin, *in_pin));

        serde_json::value::to_value(&packed)
            .into_diagnostic()
            .with_context(|| "failed to serialize graph")
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PackedGraph {
    nodes: Vec<(NodeId, PackedNode)>,
    edges: Vec<(OutPinId, InPinId)>,
    inputs: Vec<(InPinId, JsonValue)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PackedNode {
    id: Ustr,
    data: JsonValue,
    pos: Pos2,
    open: bool,
}
