use crate::graph::node::{NodeContext, SnarlNode};
use ahash::AHashMap;
use egui_snarl::{InPinId, NodeId, OutPinId, Snarl};
use itertools::Itertools;
use petgraph::acyclic::AcyclicEdgeError;
use petgraph::data::Build;
use petgraph::prelude::{EdgeRef, NodeIndex};
use petgraph::visit::IntoEdges;
use std::collections::hash_map::Entry;
use thiserror::Error;
use uuid::Uuid;

pub fn calculate_region_graph() {}

#[derive(Debug)]
pub struct RegionGraph {
    regions: Vec<RegionGraphData>,
    region_ids: AHashMap<Uuid, usize>,
    regions_by_node: AHashMap<NodeId, usize>,
}

#[derive(Debug)]
struct RegionGraphData {
    id: Uuid,
    nodes: Vec<NodeWithSeparation>,
}

/// Node that belongs to the region with a specified degree of separation
///
/// `separation` of 0 means that node belongs to region directly, 1 means the
/// node belongs to region's direct child, etc.
#[derive(Debug)]
pub struct NodeWithSeparation {
    pub node: NodeId,
    pub separation: usize,
}

pub fn build_regions_graph(
    snarl: &Snarl<SnarlNode>,
    context: NodeContext,
) -> Result<RegionGraph, GraphRegionBuildError> {
    let mut outputs = AHashMap::<NodeId, Vec<(OutPinId, InPinId)>>::new();

    let mut node_graph = petgraph::acyclic::Acyclic::<petgraph::graph::DiGraph<NodeId, ()>>::new();
    let mut node_mapping = AHashMap::new();
    for (node, _) in snarl.node_ids() {
        node_mapping.insert(node, node_graph.add_node(node));
    }
    for (out_pin, in_pin) in snarl.wires() {
        outputs
            .entry(out_pin.node)
            .or_default()
            .push((out_pin, in_pin));
        let out_node = &node_mapping[&out_pin.node];
        let to_node = &node_mapping[&in_pin.node];
        match node_graph.try_add_edge(*out_node, *to_node, ()) {
            Ok(_) => {}
            Err(err) => match err {
                AcyclicEdgeError::Cycle(cycle) => {
                    return Err(GraphRegionBuildError::NodeCycle(
                        out_pin.node,
                        node_graph[cycle.node_id()],
                    ))
                }
                AcyclicEdgeError::SelfLoop => {
                    return Err(GraphRegionBuildError::NodeSelfLoop(out_pin.node))
                }
                AcyclicEdgeError::InvalidEdge => {
                    unreachable!("Edge should be valid")
                }
            },
        }
    }

    RegionGraphBuilder::new(snarl, context, outputs).build()
}

#[derive(Debug)]
struct RegionGraphBuilder<'a> {
    /// Reference to the graph
    snarl: &'a Snarl<SnarlNode>,
    context: NodeContext<'a>,
    /// usize mappings for faster region access
    region_ids: AHashMap<Uuid, usize>,
    region_data: Vec<RegionBuilderData>,
    node_data: AHashMap<NodeId, RegionBuilderNode>,
    /// Output connections of the nodes
    outputs: AHashMap<NodeId, Vec<(OutPinId, InPinId)>>,
}

#[derive(Debug)]
struct RegionBuilderData {
    id: Uuid,
    source: NodeId,
    endpoint: NodeId,
    nodes: Vec<NodeId>,
    parents: Vec<usize>,
    toposort_index: Option<usize>,
}

#[derive(Debug, Default)]
struct RegionBuilderNode {
    source_of: Option<usize>,
    endpoint_of: Option<usize>,
    input_regions: Vec<usize>,
}

impl<'a> RegionGraphBuilder<'a> {
    fn new(
        snarl: &'a Snarl<SnarlNode>,
        context: NodeContext<'a>,
        outputs: AHashMap<NodeId, Vec<(OutPinId, InPinId)>>,
    ) -> Self {
        Self {
            snarl,
            context,
            region_ids: Default::default(),
            region_data: Default::default(),
            node_data: Default::default(),
            outputs,
        }
    }

    fn build(mut self) -> Result<RegionGraph, GraphRegionBuildError> {
        self.calculate_initial_regions()?;
        self.calculate_node_inputs()?;
        self.calculate_hierarchy()?;
        self.assign_nodes()
    }
}

impl RegionGraphBuilder<'_> {
    /// Calculates start and end points for regions
    fn calculate_initial_regions(&mut self) -> Result<(), GraphRegionBuildError> {
        for (id, node) in self.snarl.nodes_ids_data() {
            if let Some(region) = node.value.region_end(self.context) {
                match self.region_ids.entry(region) {
                    Entry::Occupied(e) => {
                        let reg_id = *e.get();
                        let data = &mut self.region_data[reg_id];
                        if data.endpoint != NodeId(usize::MAX) {
                            return Err(GraphRegionBuildError::MultipleEndpoints(
                                region,
                                data.endpoint,
                                id,
                            ));
                        }
                        data.endpoint = id;
                    }
                    Entry::Vacant(e) => {
                        e.insert(self.region_data.len());
                        self.region_data.push(RegionBuilderData {
                            id: region,
                            source: NodeId(usize::MAX),
                            endpoint: id,
                            nodes: vec![],
                            parents: vec![],
                            toposort_index: None,
                        })
                    }
                }
            };
            if let Some(region) = node.value.region_source(self.context) {
                match self.region_ids.entry(region) {
                    Entry::Occupied(e) => {
                        let reg_id = *e.get();
                        let data = &mut self.region_data[reg_id];
                        if data.source != NodeId(usize::MAX) {
                            return Err(GraphRegionBuildError::MultipleSources(
                                region,
                                data.source,
                                id,
                            ));
                        }
                        data.source = id;
                    }
                    Entry::Vacant(e) => {
                        e.insert(self.region_data.len());
                        self.region_data.push(RegionBuilderData {
                            id: region,
                            source: id,
                            endpoint: NodeId(usize::MAX),
                            nodes: vec![],
                            parents: vec![],
                            toposort_index: None,
                        })
                    }
                }
            };
        }

        for (idx, data) in self.region_data.iter().enumerate() {
            if data.source == NodeId(usize::MAX) {
                return Err(GraphRegionBuildError::MissingSource(data.id, data.endpoint));
            } else if data.endpoint == NodeId(usize::MAX) {
                return Err(GraphRegionBuildError::MissingEndpoint(data.id, data.source));
            } else if data.source == data.endpoint {
                return Err(GraphRegionBuildError::SameSourceAndEndpoint(
                    data.id,
                    data.source,
                ));
            }

            self.node_data.entry(data.source).or_default().source_of = Some(idx);
            self.node_data.entry(data.endpoint).or_default().endpoint_of = Some(idx);
        }

        Ok(())
    }

    fn calculate_node_inputs(&mut self) -> Result<(), GraphRegionBuildError> {
        fn fill_inputs_recursive(
            outputs: &AHashMap<NodeId, Vec<(OutPinId, InPinId)>>,
            nodes: &mut AHashMap<NodeId, RegionBuilderNode>,
            region: usize,
            node: NodeId,
        ) {
            let data = nodes.entry(node).or_default();
            if !data.input_regions.contains(&region) {
                data.input_regions.push(region);
            }

            // endpoint reached, stop propagation
            if data.endpoint_of == Some(region) {
                return;
            }

            if let Some(connections) = outputs.get(&node) {
                for (_, in_pin) in connections {
                    fill_inputs_recursive(outputs, nodes, region, in_pin.node);
                }
            }
        }

        for (idx, region) in self.region_data.iter().enumerate() {
            let start = region.source;

            fill_inputs_recursive(&self.outputs, &mut self.node_data, idx, start);

            // specifically paint group output, in case if it isn't connected
            let data = self.node_data.entry(region.endpoint).or_default();
            if !data.input_regions.contains(&idx) {
                data.input_regions.push(idx);
            }
        }

        Ok(())
    }

    fn calculate_hierarchy(&mut self) -> Result<(), GraphRegionBuildError> {
        let mut region_dag =
            petgraph::acyclic::Acyclic::<petgraph::graph::DiGraph<usize, ()>>::new();

        let graph_indices = (0..self.region_data.len())
            .map(|idx| region_dag.add_node(idx))
            .collect_vec();

        for (idx, data) in self.region_data.iter().enumerate() {
            let current_region = graph_indices[idx];
            let endpoint = data.endpoint;

            let node_info = self
                .node_data
                .get(&endpoint)
                .expect("Node info for endpoints was populated at calculate_initial_regions step");

            for &region in &node_info.input_regions {
                if region == idx {
                    continue;
                }

                let input_region = graph_indices[region];

                if let Err(err) = region_dag.try_add_edge(current_region, input_region, ()) {
                    match err {
                        AcyclicEdgeError::Cycle(cycle) => {
                            let cycle_node_idx = cycle.node_id();
                            let cycle_idx = graph_indices
                                .iter()
                                .position(|x| x == &cycle_node_idx)
                                .expect("All indices should be present");
                            let cyclic_data = &self.region_data[cycle_idx];
                            return Err(GraphRegionBuildError::CyclicalDependency {
                                a: (data.id, data.source),
                                b: (cyclic_data.id, cyclic_data.source),
                            });
                        }
                        AcyclicEdgeError::SelfLoop => {
                            unreachable!("check for same region happens before insertion")
                        }
                        AcyclicEdgeError::InvalidEdge => {
                            unreachable!("all edges should be valid")
                        }
                    }
                }
            }
        }

        // constructed graph has form where child regions point at parent regions

        // topologically reduce next

        let topo = petgraph::algo::toposort(&region_dag, None)
            .expect("toposort for acyclic graph should not fail");

        let (res, revmap) = petgraph::algo::tred::dag_to_toposorted_adjacency_list::<_, NodeIndex>(
            &region_dag,
            &topo,
        );

        let (reduced, _closure) = petgraph::algo::tred::dag_transitive_reduction_closure(&res);

        for node in reduced.node_indices() {
            let parents = reduced.edges(node);
            let cur_region_id = region_dag[revmap[node.index()]];
            self.region_data[cur_region_id].toposort_index = Some(node.index());

            // check that each node only has one parent
            match parents.at_most_one() {
                Ok(parent) => {
                    let Some(parent) = parent else {
                        continue;
                    };

                    let parent_region_id = region_dag[revmap[parent.target().index()]];

                    // We are topologically sorted, so nodes will come before their dependencies, which means we can fill them as we go
                    self.region_data[cur_region_id]
                        .parents
                        .push(parent_region_id);
                    for data in self.region_data.iter_mut() {
                        if data.parents.last() == Some(&cur_region_id) {
                            data.parents.push(parent_region_id);
                        }
                    }
                }

                Err(parents) => {
                    let cur_region = &self.region_data[cur_region_id];
                    return Err(GraphRegionBuildError::MultipleParentRegions {
                        region: (cur_region.id, cur_region.source),
                        parents: parents
                            .map(|r| {
                                let reg = &self.region_data[region_dag[revmap[r.target().index()]]];
                                (reg.id, reg.source)
                            })
                            .collect_vec(),
                    });
                }
            }
        }

        #[cfg(debug_assertions)]
        for reg in self.region_data.iter_mut() {
            reg.toposort_index
                .as_ref()
                .expect("Toposort index should be set for all regions");
        }

        Ok(())
    }

    /// Assigns nodes to the regions based on computed hierarchy
    fn assign_nodes(&mut self) -> Result<RegionGraph, GraphRegionBuildError> {
        let mut graph = RegionGraph {
            regions: self
                .region_data
                .iter()
                .map(|reg| RegionGraphData {
                    id: reg.id,
                    nodes: vec![],
                })
                .collect(),
            region_ids: self.region_ids.clone(),
            regions_by_node: Default::default(),
        };

        for (id, data) in self.node_data.iter_mut() {
            if data.input_regions.is_empty() {
                continue;
            }

            // reduce regions to 1 by eliminating nodes that came earlier in hierarchy
            if data.input_regions.len() > 1 {
                // sort inputs topologically
                data.input_regions.sort_unstable_by_key(|reg| {
                    #[cfg(debug_assertions)]
                    {
                        self.region_data[*reg]
                            .toposort_index
                            .expect("Toposort index should have been set in `calculate_hierarchy`")
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        self.region_data[*reg].toposort_index
                    }
                });

                const DELETED_REGION: usize = usize::MAX;

                for i in 0..data.input_regions.len() {
                    let region_id = data.input_regions[i];
                    if region_id == DELETED_REGION {
                        continue;
                    }

                    // Parents are listed in the same order as sorted nodes,
                    // so we can delete them by iterating linearly
                    //
                    // We can't zip directly here, since some ancestor regions
                    // may be skipped in this listing
                    let mut j = i + 1;
                    for ancestor_region in &self.region_data[region_id].parents {
                        if j >= data.input_regions.len() {
                            break;
                        }
                        let node_region = &mut data.input_regions[j];
                        if ancestor_region == node_region {
                            *node_region = DELETED_REGION;
                            j += 1;
                        }
                    }
                }

                data.input_regions.retain(|r| *r != DELETED_REGION);

                if data.input_regions.len() > 1 {
                    return Err(GraphRegionBuildError::AmbiguousNodeRegion {
                        node: *id,
                        regions: data
                            .input_regions
                            .iter()
                            .map(|r| {
                                let data = &self.region_data[*r];

                                (data.id, data.source)
                            })
                            .collect(),
                    });
                }
            }

            let node_region = data.input_regions[0];
            graph.regions_by_node.insert(*id, node_region);

            for (separation, region) in [node_region]
                .into_iter()
                .chain(self.region_data[node_region].parents.iter().copied())
                .enumerate()
            {
                graph.regions[region].nodes.push(NodeWithSeparation {
                    node: *id,
                    separation,
                });
            }
        }

        Ok(graph)
    }
}

#[derive(Debug, Clone, Error)]
pub enum GraphRegionBuildError {
    #[error("Node {:?} forms a loop with node {:?}", .0, .1)]
    NodeCycle(NodeId, NodeId),
    #[error("Node {:?} forms a loop with itself", .0)]
    NodeSelfLoop(NodeId),
    #[error("Node {:?} belongs to unknown region: {}", .1, .0)]
    UnknownRegion(Uuid, NodeId),
    #[error("Region ends in nodes {:?} and {:?}: {}", .1, .2, .0)]
    MultipleEndpoints(Uuid, NodeId, NodeId),
    #[error("Region starts in nodes {:?} and {:?}: {}", .1, .2, .0)]
    MultipleSources(Uuid, NodeId, NodeId),
    #[error("Region started in node {:?} has no endpoint node: {}", .1, .0)]
    MissingEndpoint(Uuid, NodeId),
    #[error("Region ended in node {:?} has no source node: {}", .1, .0)]
    MissingSource(Uuid, NodeId),
    #[error("Node {:?} acts as both source and endpoint for region: {}", .1, .0)]
    SameSourceAndEndpoint(Uuid, NodeId),
    #[error("Node {:?} is connected to multiple regions: {}", .node, format_region_ids(.regions)
    )]
    AmbiguousNodeRegion {
        node: NodeId,
        regions: Vec<(Uuid, NodeId)>,
    },
    #[error("Regions form a cycle: {}, {}", format_region_with_id(.a), format_region_with_id(.b))]
    CyclicalDependency {
        a: (Uuid, NodeId),
        b: (Uuid, NodeId),
    },
    #[error("Region {} has multiple parent regions: {}", format_region_with_id(.region), format_region_ids(.parents))]
    MultipleParentRegions {
        region: (Uuid, NodeId),
        parents: Vec<(Uuid, NodeId)>,
    },
}

fn format_region(id: Uuid) -> String {
    id.to_string()[..8].to_string()
}

fn format_region_ids(regions: &Vec<(Uuid, NodeId)>) -> String {
    regions
        .iter()
        .map(|(id, node)| format!("{} ({:?})", format_region(*id), *node))
        .join(", ")
}

fn format_region_with_id((id, node): &(Uuid, NodeId)) -> String {
    format!("{} ({:?})", format_region(*id), *node)
}
