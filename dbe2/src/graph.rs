use crate::graph::node::{get_snarl_node, SnarlNode};
use crate::json_utils::JsonValue;
use crate::m_try;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use ahash::AHashMap;
use egui_snarl::{InPinId, NodeId, OutPinId, Snarl};
use emath::Pos2;
use itertools::Itertools;
use miette::{bail, miette, Context, IntoDiagnostic};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::BTreeMap;
use std::fmt::Debug;
use ustr::Ustr;

pub mod node;

#[derive(Debug)]
pub struct Graph {
    snarl: Snarl<SnarlNode>,
    inputs: AHashMap<InPinId, EValue>,

    dirty: RwLock<Vec<NodeId>>,
    cache: AHashMap<NodeId, Vec<EValue>>,
    locked: bool,
}

impl Graph {
    pub fn eval(&mut self, registry: &ETypesRegistry, full_eval: bool) -> miette::Result<()> {
        struct Workload<'a> {
            snarl: &'a Snarl<SnarlNode>,
            inputs: &'a mut AHashMap<InPinId, EValue>,
            registry: &'a ETypesRegistry,
            cache: &'a mut AHashMap<NodeId, Vec<EValue>>,
            stack: &'a mut Vec<NodeId>,
        }

        let mut dirty = self.dirty.write();
        for i in dirty.drain(..) {
            self.cache.remove(&i);
        }
        drop(dirty);
        self.locked = true;

        for (id, has_side_effects) in self
            .snarl
            .node_ids()
            .map(|(id, node)| (id, node.has_side_effects()))
            .collect_vec()
        {
            if !has_side_effects && !full_eval {
                continue;
            }
            if self.cache.contains_key(&id) {
                continue;
            }

            let mut stack = Vec::new();

            self.eval_node_inner(registry, &mut stack, id)?
        }

        Ok(())
    }

    pub fn read_output(
        &mut self,
        registry: &ETypesRegistry,
        id: OutPinId,
    ) -> miette::Result<EValue> {
        let mut stack = Vec::new();
        self.read_node_output_inner(registry, &mut stack, id)
    }

    pub fn read_input(&mut self, registry: &ETypesRegistry, id: InPinId) -> miette::Result<EValue> {
        let node = dyn_clone::clone_box(
            self.snarl
                .get_node(id.node)
                .ok_or_else(|| miette!("Node {:?} not found", id.node))?
                .as_ref(),
        );
        let mut stack = Vec::new();
        self.read_node_input_inner(registry, &mut stack, id, &node)
    }

    pub fn mark_dirty(&self, id: NodeId) -> miette::Result<()> {
        if self.locked {
            bail!("Graph is locked");
        }
        self.dirty.write().push(id);
        Ok(())
    }

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

            let node_inputs = node
                .inputs(registry)
                .with_context(|| format!("failed to get inputs for node {:?}", in_pin.node))?;
            if in_pin.input >= node_inputs.len() {
                bail!(
                    "input index #{} out of bounds for node {:?}",
                    in_pin.input,
                    in_pin.node
                );
            }
            let input_type = node_inputs[in_pin.input];
            let value = input_type
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
            dirty: Default::default(),
            cache: Default::default(),
            locked: false,
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

impl Graph {
    fn read_node_output_inner(
        &mut self,
        registry: &ETypesRegistry,
        stack: &mut Vec<NodeId>,
        pin: OutPinId,
    ) -> miette::Result<EValue> {
        m_try(|| {
            if let Some(node) = self.cache.get(&pin.node) {
                return Ok(node
                    .get(pin.output)
                    .ok_or_else(|| miette!("Node doesn't have output #{}", pin.output))?
                    .clone());
            }

            self.eval_node_inner(registry, stack, pin.node)?;

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

    fn read_node_input_inner(
        &mut self,
        registry: &ETypesRegistry,
        stack: &mut Vec<NodeId>,
        id: InPinId,
        node: &SnarlNode,
    ) -> miette::Result<EValue> {
        m_try(|| {
            // TODO: check for valid types
            let slot = self.snarl.in_pin(id);
            let value = if slot.remotes.is_empty() {
                match self.inputs.entry(slot.id) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => {
                        let default = node.default_input_value(registry, id.input)?;
                        entry.insert(default.into_owned())
                    }
                }
                .clone()
            } else if slot.remotes.len() == 1 {
                let remote = slot.remotes[0];
                self.read_node_output_inner(registry, stack, remote)?
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
        registry: &ETypesRegistry,
        stack: &mut Vec<NodeId>,
        id: NodeId,
    ) -> miette::Result<()> {
        m_try(|| {
            // let registry= self.registry;
            // if self.stack.contains(&id) {
            //     bail!("Cyclic dependency detected");
            // }
            // self.stack.push(id);

            let node = dyn_clone::clone_box(
                self.snarl
                    .get_node(id)
                    .ok_or_else(|| miette!("Node {:?} not found", id))?
                    .as_ref(),
            );

            let inputs = node.inputs(registry)?;
            let mut input_values = Vec::<EValue>::with_capacity(inputs.len());

            for i in 0..inputs.len() {
                input_values.push(self.read_node_input_inner(
                    registry,
                    stack,
                    InPinId { node: id, input: i },
                    &node,
                )?);
            }

            let out_types = node.outputs(registry)?;
            let mut outputs = Vec::with_capacity(out_types.len());
            node.execute(registry, &input_values, &mut outputs)?;

            // TODO: check for validity of returned values types
            self.cache.insert(id, outputs);

            Ok(())
        })
        .with_context(|| format!("failed to evaluate node {:?}", id))
    }
}
