use crate::graph::cache::GraphCache;
use crate::graph::inputs::{GraphInput, GraphOutput};
use crate::graph::Graph;
use crate::json_utils::JsonValue;
use crate::project::ProjectFile;
use crate::registry::ETypesRegistry;
use ahash::AHashMap;
use miette::{Context, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use strum::EnumIs;
use uuid::Uuid;

#[derive(Debug)]
pub struct NodeGroup {
    pub nodes: AHashMap<Uuid, Graph>,
}

#[derive(Debug)]
pub struct ProjectGraph {
    pub id: Uuid,
    pub name: Option<String>,
    graph: GraphHolder,
    inputs_cache: SmallVec<[GraphInput; 1]>,
    outputs_cache: SmallVec<[GraphOutput; 1]>,
}

#[derive(Debug, EnumIs)]
enum GraphHolder {
    Graph(Box<Graph>),
    Editing,
}

impl ProjectGraph {
    /// Get the graph data
    pub fn graph(&self) -> &Graph {
        match &self.graph {
            GraphHolder::Graph(g) => g,
            GraphHolder::Editing => panic!("Cannot borrow graph: graph is being edited"),
        }
    }

    /// Get the graph inputs
    pub fn inputs(&self) -> &[GraphInput] {
        match self.graph {
            GraphHolder::Graph(ref g) => g.inputs(),
            GraphHolder::Editing => &self.inputs_cache,
        }
    }

    /// Get the graph outputs
    pub fn outputs(&self) -> &[GraphOutput] {
        match self.graph {
            GraphHolder::Graph(ref g) => g.outputs(),
            GraphHolder::Editing => &self.outputs_cache,
        }
    }

    pub fn parse_json(registry: &ETypesRegistry, value: &mut JsonValue) -> miette::Result<Self> {
        let data = if value.get("version").is_none() {
            SerializedGraphRepr::V0(value.take())
        } else {
            SerializedGraphRepr::deserialize(value.take()).into_diagnostic()?
        };

        match data {
            SerializedGraphRepr::V1(mut data) => {
                let graph = Graph::parse_json(registry, &mut data.graph)
                    .context("failed to deserialize graph data")?;

                Ok(Self {
                    id: data.id,
                    name: data.name,
                    graph: GraphHolder::Graph(Box::new(graph)),
                    inputs_cache: Default::default(),
                    outputs_cache: Default::default(),
                })
            }
            SerializedGraphRepr::V0(mut data) => {
                // Legacy raw graph data

                if let Some(obj) = data.as_object_mut() {
                    // Patch renamed field
                    if let Some(inputs) = obj.remove("inputs") {
                        obj.insert("inline_values".to_string(), inputs);
                    }
                }

                let graph = Graph::parse_json(registry, dbg!(&mut data))
                    .context("failed to deserialize graph data")?;
                Ok(Self {
                    id: Uuid::new_v4(),
                    name: None,
                    graph: GraphHolder::Graph(Box::new(graph)),
                    inputs_cache: Default::default(),
                    outputs_cache: Default::default(),
                })
            }
        }
    }

    pub fn write_json(&self, registry: &ETypesRegistry) -> miette::Result<JsonValue> {
        let graph = self.graph().write_json(registry)?;

        let serialized = SerializedGraphRepr::V1(PackedProjectGraph {
            id: self.id,
            name: self.name.clone(),
            graph,
        });

        serde_json::value::to_value(&serialized)
            .into_diagnostic()
            .with_context(|| "failed to serialize graph")
    }

    fn take_graph(&mut self) -> Box<Graph> {
        match std::mem::replace(&mut self.graph, GraphHolder::Editing) {
            GraphHolder::Graph(g) => {
                self.inputs_cache = g.inputs().clone();
                self.outputs_cache = g.outputs().clone();
                g
            }
            GraphHolder::Editing => panic!("Cannot take graph: graph is being edited"),
        }
    }

    fn return_graph(&mut self, graph: Box<Graph>) {
        if !self.graph.is_editing() {
            panic!("Cannot return graph: graph is not being edited");
        }
        self.graph = GraphHolder::Graph(graph);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PackedProjectGraph {
    id: Uuid,
    #[serde(default)]
    name: Option<String>,
    graph: JsonValue,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "version")]
enum SerializedGraphRepr {
    V1(PackedProjectGraph),
    V0(JsonValue),
}

#[derive(Debug, Default)]
pub struct ProjectGraphs {
    /// All project graphs
    pub graphs: AHashMap<Uuid, ProjectGraph>,
    /// Cache for project graphs. Should only be used when executing graph standalone, not as a node group
    pub cache: AHashMap<Uuid, GraphCache>,
}

impl ProjectGraphs {
    pub fn edit_graph<Fn: FnOnce(&mut Graph, &mut GraphCache, &Self) -> R, R>(
        &mut self,
        id: Uuid,
        func: Fn,
    ) -> R {
        // Take the graph data out of the
        //         let graph = self.graphs.get_mut(&id).unwrap();
        let graph = self.graphs.get_mut(&id).unwrap();
        let mut taken = graph.take_graph();
        let mut cache = self.cache.remove(&id).unwrap_or_default();

        // Run the callback
        let result = func(&mut taken, &mut cache, self);

        // Return the graph data to the project
        self.cache.insert(id, cache);
        let graph = self.graphs.get_mut(&id).unwrap();
        graph.return_graph(taken);
        result
    }

    pub fn add_graph(&mut self, graph: ProjectGraph) -> ProjectFile {
        let id = graph.id;
        self.graphs.insert(id, graph);

        ProjectFile::Graph(id)
    }
}
