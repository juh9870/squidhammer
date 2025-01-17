use crate::graph::node::SnarlNode;
use egui_snarl::{InPinId, NodeId, OutPinId, Snarl};
use itertools::Itertools;
use miette::Diagnostic;
use petgraph::acyclic::AcyclicEdgeError;
use petgraph::data::Build;
use petgraph::prelude::{EdgeRef, NodeIndex};
use petgraph::visit::IntoEdges;
use smallvec::SmallVec;
use std::collections::hash_map::Entry;
use std::iter::once;
use std::marker::PhantomData;
use thiserror::Error;
use utils::map::HashMap;
use uuid::Uuid;

#[derive(Debug)]
pub struct RegionGraph(Result<RegionGraphData, GraphRegionBuildError>);

impl RegionGraph {
    /// Specifies whenever this region graph was build (to either success or error)
    ///
    /// Graph can be ready while having a build error, use
    /// [RegionGraph::try_as_data] to check for build errors
    pub fn is_ready(&self) -> bool {
        !matches!(
            self.0,
            Err(GraphRegionBuildError::NotReady | GraphRegionBuildError::Dirty)
        )
    }

    /// Returns an error is graph is not ready
    ///
    /// Graph can be ready while having a build error, use
    /// [RegionGraph::try_as_data] to check for build errors
    pub fn expect_ready(&self) -> Result<(), GraphRegionBuildError> {
        match self.0.as_ref() {
            Err(err @ (GraphRegionBuildError::NotReady | GraphRegionBuildError::Dirty)) => {
                Err(err.clone())
            }
            _ => Ok(()),
        }
    }

    /// Marks region hierarchy as dirty, and requiring a rebuild
    pub fn mark_dirty(&mut self) {
        self.0 = Err(GraphRegionBuildError::Dirty)
    }

    /// Attempts to get a reference to the underlying region graph
    pub fn try_as_data(&self) -> Result<&RegionGraphData, GraphRegionBuildError> {
        self.0.as_ref().map_err(|err| err.clone())
    }

    /// Forces a rebuild in the region graph
    pub fn force_rebuild(&mut self, snarl: &Snarl<SnarlNode>) {
        *self = Self::build_regions_graph(snarl);
    }

    /// Rebuilds the region graph if it's not ready
    pub fn ensure_ready(&mut self, snarl: &Snarl<SnarlNode>) {
        if !self.is_ready() {
            self.force_rebuild(snarl);
        }
    }
}

impl RegionGraph {
    /// Attempts to build a region graph for the given node graph
    pub fn build_regions_graph(snarl: &Snarl<SnarlNode>) -> Self {
        let mut outputs = HashMap::<NodeId, Vec<(OutPinId, InPinId)>>::default();

        let mut node_graph =
            petgraph::acyclic::Acyclic::<petgraph::graph::DiGraph<NodeId, ()>>::new();
        let mut node_mapping = HashMap::default();
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
                        return Self(Err(GraphRegionBuildError::NodeCycle(
                            out_pin.node,
                            node_graph[cycle.node_id()],
                        )))
                    }
                    AcyclicEdgeError::SelfLoop => {
                        return Self(Err(GraphRegionBuildError::NodeSelfLoop(out_pin.node)))
                    }
                    AcyclicEdgeError::InvalidEdge => {
                        unreachable!("Edge should be valid")
                    }
                },
            }
        }

        Self(RegionGraphBuilder::new(snarl, outputs).build())
    }
}

impl Default for RegionGraph {
    fn default() -> Self {
        Self(Err(GraphRegionBuildError::NotReady))
    }
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

#[derive(Debug)]
pub struct RegionGraphData {
    topological_order: Vec<Uuid>,
    regions: HashMap<Uuid, RegionGraphRegionData>,
    regions_by_node: HashMap<NodeId, Uuid>,
}

impl RegionGraphData {
    /// Regions ordered in a way, such that region children come after their parents in the iteration order
    pub fn ordered_regions(&self) -> &[Uuid] {
        &self.topological_order
    }

    /// Returns all nodes that belong to the region or child regions
    pub fn region_nodes(&self, region: &Uuid) -> &[NodeWithSeparation] {
        &self.regions[region].nodes
    }

    pub fn region_data(&self, region: &Uuid) -> &RegionGraphRegionData {
        &self.regions[region]
    }

    /// Returns the topmost region that the node belongs to
    pub fn node_region(&self, node: &NodeId) -> Option<Uuid> {
        self.regions_by_node.get(node).copied()
    }

    /// Returns hierarchy for the region
    pub fn region_parents(&self, region: &Uuid) -> &[Uuid] {
        &self.regions[region].parents
    }
}

#[derive(Debug)]
pub struct RegionGraphRegionData {
    pub parents: Vec<Uuid>,
    pub nodes: Vec<NodeWithSeparation>,
    pub has_side_effects: bool,
    pub start_node: NodeId,
    pub end_node: NodeId,
    _marker: PhantomData<()>, // make struct non-constructable
}

#[derive(Debug)]
struct RegionGraphBuilder<'a> {
    /// Reference to the graph
    snarl: &'a Snarl<SnarlNode>,
    /// usize mappings for faster region access
    region_ids: HashMap<Uuid, usize>,
    region_data: Vec<RegionBuilderData>,
    node_data: HashMap<NodeId, RegionBuilderNode>,
    /// Output connections of the nodes
    outputs: HashMap<NodeId, Vec<(OutPinId, InPinId)>>,
}

#[derive(Debug)]
struct RegionBuilderData {
    id: Uuid,
    source: NodeId,
    endpoint: NodeId,
    parents: Vec<usize>,
    toposort_index: Option<usize>,
}

#[derive(Debug, Default)]
struct RegionBuilderNode {
    source_of: Option<usize>,
    endpoint_of: Option<usize>,
    input_regions: SmallVec<[usize; 2]>,
}

impl<'a> RegionGraphBuilder<'a> {
    fn new(
        snarl: &'a Snarl<SnarlNode>,
        outputs: HashMap<NodeId, Vec<(OutPinId, InPinId)>>,
    ) -> Self {
        Self {
            snarl,
            region_ids: Default::default(),
            region_data: Default::default(),
            node_data: Default::default(),
            outputs,
        }
    }

    fn build(mut self) -> Result<RegionGraphData, GraphRegionBuildError> {
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
            if let Some(region) = node.value.region_end() {
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
                            parents: vec![],
                            toposort_index: None,
                        })
                    }
                }
            };
            if let Some(region) = node.value.region_source() {
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
            outputs: &HashMap<NodeId, Vec<(OutPinId, InPinId)>>,
            nodes: &mut HashMap<NodeId, RegionBuilderNode>,
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
        }

        for (idx, region) in self.region_data.iter().enumerate() {
            let source_regions = self
                .node_data
                .get(&region.source)
                .expect("All region sources were populated")
                .input_regions
                .clone();

            // specifically paint group output, in case if it isn't connected
            let endpoint_data = self.node_data.entry(region.endpoint).or_default();
            for reg in once(idx).chain(source_regions) {
                if !endpoint_data.input_regions.contains(&reg) {
                    endpoint_data.input_regions.push(reg);
                }
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

        // transitively reduce next

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
    fn assign_nodes(&mut self) -> Result<RegionGraphData, GraphRegionBuildError> {
        let mut graph = RegionGraphData {
            topological_order: self
                .region_data
                .iter()
                .sorted_unstable_by_key(|reg| {
                    #[cfg(debug_assertions)]
                    {
                        reg.toposort_index
                            .expect("Toposort index should have been set in `calculate_hierarchy`")
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        reg.toposort_index
                    }
                })
                .rev()
                .map(|reg| reg.id)
                .collect(),
            regions: self
                .region_data
                .iter()
                .map(|reg| {
                    (
                        reg.id,
                        RegionGraphRegionData {
                            parents: reg
                                .parents
                                .iter()
                                .map(|id| self.region_data[*id].id)
                                .collect(),
                            nodes: vec![],
                            has_side_effects: false,
                            start_node: reg.source,
                            end_node: reg.endpoint,
                            _marker: Default::default(),
                        },
                    )
                })
                .collect(),
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
            graph
                .regions_by_node
                .insert(*id, self.region_data[node_region].id);

            let has_side_effects = self.snarl[*id].has_side_effects();

            for (separation, region) in [node_region]
                .into_iter()
                .chain(self.region_data[node_region].parents.iter().copied())
                .enumerate()
            {
                let reg = graph
                    .regions
                    .get_mut(&self.region_data[region].id)
                    .expect("all regions were populated on construction");

                reg.nodes.push(NodeWithSeparation {
                    node: *id,
                    separation,
                });

                if has_side_effects {
                    reg.has_side_effects = true;
                }
            }
        }

        Ok(graph)
    }
}

#[derive(Debug, Clone, Error, Diagnostic)]
pub enum GraphRegionBuildError {
    #[error("Region graph was not built yet")]
    NotReady,
    #[error("Region graph is dirty")]
    Dirty,
    #[error("Node {:?} forms a loop with node {:?}", .0, .1)]
    NodeCycle(NodeId, NodeId),
    #[error("Node {:?} forms a loop with itself", .0)]
    NodeSelfLoop(NodeId),
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
    #[error("Region {} has multiple parent regions: {}",
        format_region_with_id(.region),
        format_region_ids(.parents)
    )]
    MultipleParentRegions {
        region: (Uuid, NodeId),
        parents: Vec<(Uuid, NodeId)>,
    },
}

fn format_region(id: Uuid) -> String {
    id.to_string()[..8].to_string()
}

fn format_region_ids(regions: &[(Uuid, NodeId)]) -> String {
    regions
        .iter()
        .map(|(id, node)| format!("{} ({:?})", format_region(*id), *node))
        .join(", ")
}

fn format_region_with_id((id, node): &(Uuid, NodeId)) -> String {
    format!("{} ({:?})", format_region(*id), *node)
}
