use crate::graph::editing::GraphEditingContext;
use crate::graph::inputs::{GraphInput, GraphOutput};
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
use smallvec::SmallVec;
use std::fmt::Debug;
use ustr::Ustr;

pub mod cache;
pub mod editing;
pub mod execution;
pub mod inputs;
pub mod node;

/// A container of a graph with inline values. It contains all the data
/// that is unique to this graph and is required for both node groups and standalone graphs
#[derive(Debug, Default)]
pub struct Graph {
    snarl: Snarl<SnarlNode>,
    inline_values: AHashMap<InPinId, EValue>,
    inputs: SmallVec<[GraphInput; 1]>,
    outputs: SmallVec<[GraphOutput; 1]>,
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

        let inputs = AHashMap::with_capacity(packed.inline_values.len());

        let mut graph = Self {
            snarl,
            inline_values: inputs,
            inputs: packed.inputs,
            outputs: packed.outputs,
        };

        m_try(|| {
            let mut side_effects = SideEffects::default();
            let mut cache = cache::GraphCache::default();
            let mut ctx = GraphEditingContext::from_graph(
                &mut graph,
                registry,
                &mut cache,
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
                let out_pin = ctx.snarl.out_pin(out_pin);
                let in_pin = ctx.snarl.in_pin(in_pin);
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
                ctx.connect(&out_pin, &in_pin, commands)?;
            }

            commands
                .execute(&mut ctx)
                .with_context(|| "failed to execute commands")?;

            if !side_effects.is_empty() {
                panic!("Side effects are not supported during deserialization");
            }

            Ok(())
        })
        .context("failed to connect pins")?;

        m_try(|| {
            for (mut in_pin, mut value) in packed.inline_values {
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

                let Some(info) = input_type.ty.item_info() else {
                    continue;
                };

                let value = info
                    .ty()
                    .parse_json(registry, &mut value, false)
                    .with_context(|| {
                        format!(
                            "failed to parse input value #{} for node {:?}",
                            in_pin.input, in_pin.node
                        )
                    })?;
                graph.inline_values.insert(in_pin, value);
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
            inline_values: Default::default(),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
        };

        let mut inline_values = AHashMap::new();

        for (pin, value) in &self.inline_values {
            let Some(node) = self.snarl.get_node(pin.node) else {
                continue;
            };

            if !node.has_inline_values()? {
                continue;
            }

            let value_json = value.write_json(registry).with_context(|| {
                format!(
                    "failed to serialize input value #{} for {:?}",
                    pin.input, pin.node
                )
            })?;
            inline_values.insert(*pin, value_json);
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
            inline_values.remove(&in_pin);
            packed.edges.push((out_pin, in_pin));
        }

        packed.inline_values = inline_values.into_iter().collect();
        packed.inline_values.sort_by_key(|(in_pin, _)| *in_pin);
        packed.nodes.sort_by_key(|(id, _)| *id);
        packed
            .edges
            .sort_by_key(|(out_pin, in_pin)| (*out_pin, *in_pin));

        serde_json::value::to_value(&packed)
            .into_diagnostic()
            .with_context(|| "failed to serialize graph")
    }

    pub fn inputs(&self) -> &SmallVec<[GraphInput; 1]> {
        &self.inputs
    }

    pub fn outputs(&self) -> &SmallVec<[GraphOutput; 1]> {
        &self.outputs
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PackedGraph {
    nodes: Vec<(NodeId, PackedNode)>,
    edges: Vec<(OutPinId, InPinId)>,
    inline_values: Vec<(InPinId, JsonValue)>,
    #[serde(default)]
    inputs: SmallVec<[GraphInput; 1]>,
    #[serde(default)]
    outputs: SmallVec<[GraphOutput; 1]>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PackedNode {
    id: Ustr,
    data: JsonValue,
    pos: Pos2,
    open: bool,
}
