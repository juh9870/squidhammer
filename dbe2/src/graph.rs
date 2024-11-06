use crate::graph::node::{get_snarl_node, SnarlNode};
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use ahash::AHashMap;
use egui_snarl::{InPinId, NodeId, OutPinId, Snarl};
use emath::Pos2;
use miette::{miette, Context, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Debug;
use ustr::Ustr;

pub mod execution;
pub mod node;

#[derive(Debug)]
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

        for (serialized_id, node) in packed.nodes {
            let created_node = get_snarl_node(&node.id)
                .ok_or_else(|| miette!("node type {} not found", node.id))?;
            let node_id = if node.open {
                snarl.insert_node(node.pos, created_node)
            } else {
                snarl.insert_node_collapsed(node.pos, created_node)
            };
            mapping.insert(serialized_id, node_id);
        }

        for (mut out_pin, mut in_pin) in packed.edges {
            out_pin.node = *mapping
                .get(&out_pin.node)
                .ok_or_else(|| miette!("node {:?} not found", out_pin.node))?;
            in_pin.node = *mapping
                .get(&in_pin.node)
                .ok_or_else(|| miette!("node {:?} not found", in_pin.node))?;
            snarl.connect(out_pin, in_pin);
        }

        let mut inputs = AHashMap::with_capacity(packed.inputs.len());
        for (mut in_pin, mut value) in packed.inputs {
            in_pin.node = *mapping
                .get(&in_pin.node)
                .ok_or_else(|| miette!("node {:?} not found", in_pin.node))?;

            let node = snarl
                .get_node(in_pin.node)
                .expect("Mappings should be correct");

            let input_type = node
                .try_input(registry, in_pin.input)
                .with_context(|| format!("failed to get inputs for node {:?}", in_pin.node))?;
            let value = input_type
                .ty
                .parse_json(registry, &mut value, false)
                .with_context(|| {
                    format!(
                        "failed to parse input value #{} for node {:?}",
                        in_pin.input, in_pin.node
                    )
                })?;
            inputs.insert(in_pin, value);
        }

        Ok(Self {
            snarl,
            inputs,
            cache: Default::default(),
        })
    }

    pub fn write_json(&self, registry: &ETypesRegistry) -> miette::Result<JsonValue> {
        let mut packed = PackedGraph {
            nodes: Default::default(),
            edges: Default::default(),
            inputs: Default::default(),
        };

        for (pin, value) in &self.inputs {
            let value_json = value.write_json(registry).with_context(|| {
                format!(
                    "failed to serialize input value #{} for {:?}",
                    pin.input, pin.node
                )
            })?;
            packed.inputs.insert(*pin, value_json);
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
            packed.nodes.insert(id, packed_node);
        }

        for (out_pin, in_pin) in self.snarl.wires() {
            // do not serialize inputs with connections
            packed.inputs.remove(&in_pin);
            packed.edges.push((out_pin, in_pin));
        }

        serde_json::value::to_value(&packed)
            .into_diagnostic()
            .with_context(|| "failed to serialize graph")
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PackedGraph {
    nodes: BTreeMap<NodeId, PackedNode>,
    edges: Vec<(OutPinId, InPinId)>,
    inputs: AHashMap<InPinId, JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PackedNode {
    id: Ustr,
    data: JsonValue,
    pos: Pos2,
    open: bool,
}
