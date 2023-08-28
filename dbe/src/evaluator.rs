use crate::commands::Command;
use crate::nodes::EditorNode;
use crate::value::{EValue, EValueInputWrapper};
use crate::EditorGraph;
use anyhow::anyhow;
use egui_node_graph::{NodeId, NodeTemplateTrait, OutputId};
use rustc_hash::FxHashMap;
use smallvec::{smallvec, SmallVec};

// type OutputsCache = FxHashMap<OutputId, EValue>;
//
pub type InputResult<'a> = SmallVec<[&'a EValue; 1]>;
// pub struct Evaluator<'a, 'graph> {
//     pub graph: &'graph EditorGraph,
//     pub outputs_cache: OutputsCache,
// }
//
// impl<'a> Evaluator<'a> {
//     pub fn new(graph: &'a EditorGraph) -> Self {
//         Self {
//             graph,
//             outputs_cache: Default::default(),
//         }
//     }
//     pub fn evaluate_input(
//         &mut self,
//         name: &str,
//         node_id: NodeId,
//     ) -> anyhow::Result<InputResult<'_>> {
//         let input_id = self.graph[node_id].get_input(name)?;
//
//         let mut inputs: InputResult<'_> = smallvec![];
//
//         // The output of another node is connected.
//         for other_id in self.graph.connections(input_id) {
//             if !self.outputs_cache.contains_key(&other_id) {
//                 let id = self.graph[other_id].node;
//
//                 let node = self.graph.nodes[id].user_data.template;
//                 node.evaluate(self, id)?;
//
//                 inputs.push(
//                     self.outputs_cache
//                         .get(&other_id)
//                         .ok_or_else(|| anyhow!("Cache should be populated"))?,
//                 );
//             }
//
//             // The value was already computed due to the evaluation of some other
//             // node. We simply return value from the cache.
//             if let Some(other_value) = self.outputs_cache.get(&other_id) {
//                 inputs.push(other_value)
//             } else {
//                 unreachable!();
//             }
//         }
//
//         if inputs.is_empty() {
//             inputs.push(self.graph[input_id].value())
//         }
//
//         Ok(inputs)
//     }
//
//     pub fn evaluate_input_as<T: TryFrom<EValueInputWrapper<'a>, Error = anyhow::Error>>(
//         &mut self,
//         name: &str,
//         node_id: NodeId,
//     ) -> Result<T, anyhow::Error> {
//         let item = self.evaluate_input(name, node_id)?;
//         let wrapped = EValueInputWrapper::<'a>(item);
//         Ok(wrapped.try_into()?)
//     }
//
//     pub fn populate_output(
//         &'a mut self,
//         name: &str,
//         value: EValue,
//         node_id: NodeId,
//     ) -> anyhow::Result<&EValue> {
//         let output_id = self.graph[node_id].get_output(name)?;
//
//         Ok(self.outputs_cache.entry(output_id).or_insert(value))
//     }
//
//     pub fn evaluate_node(&'a mut self, id: NodeId) -> anyhow::Result<()> {
//         self.graph.nodes[id].user_data.template.evaluate(self, id)?;
//         Ok(())
//     }
// }

pub type OutputsCache = FxHashMap<OutputId, EValue>;

/// Recursively evaluates all dependencies of this node, then evaluates the node itself.
pub fn evaluate_node(
    graph: &EditorGraph,
    outputs_cache: &mut OutputsCache,
    commands: &mut Vec<Command>,
    node_id: NodeId,
) -> anyhow::Result<()> {
    let node = &graph[node_id];
    node.user_data
        .template
        .evaluate(graph, outputs_cache, commands, node_id)?;
    Ok(())
}

pub fn populate_output(
    graph: &EditorGraph,
    outputs_cache: &mut OutputsCache,
    node_id: NodeId,
    param_name: &str,
    value: EValue,
) -> anyhow::Result<()> {
    let output_id = graph[node_id].get_output(param_name)?;
    outputs_cache.insert(output_id, value);
    Ok(())
}

// Evaluates the input value of
pub fn evaluate_input<'a>(
    graph: &'a EditorGraph,
    outputs_cache: &'a mut OutputsCache,
    commands: &mut Vec<Command>,
    node_id: NodeId,
    name: &str,
) -> anyhow::Result<InputResult<'a>> {
    let input_id = graph[node_id].get_input(name)?;

    let mut inputs: SmallVec<[&'a EValue; 1]> = smallvec![];

    let ids = graph.connections(input_id);
    // The output of another node is connected.
    for other_id in ids.iter() {
        if !outputs_cache.contains_key(other_id) {
            let other_node = graph.outputs[*other_id].node;
            evaluate_node(graph, outputs_cache, commands, other_node)?;
        }
    }

    for other_id in ids {
        inputs.push(
            outputs_cache
                .get(&other_id)
                .ok_or_else(|| anyhow!("Cache should be populated"))?,
        );
    }

    if inputs.is_empty() {
        inputs.push(graph[input_id].value())
    }

    Ok(inputs)
}

pub fn evaluate_input_as<'a, T: TryFrom<EValueInputWrapper<'a>, Error = anyhow::Error>>(
    graph: &'a EditorGraph,
    outputs_cache: &'a mut OutputsCache,
    commands: &mut Vec<Command>,
    node_id: NodeId,
    name: &str,
) -> anyhow::Result<T> {
    let data = evaluate_input(graph, outputs_cache, commands, node_id, name)?;
    EValueInputWrapper(data).try_into()
}
