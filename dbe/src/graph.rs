use crate::graph::evaluator::evaluate_node;
use crate::value::etype::EDataType;
use crate::value::EValue;
use commands::Command;
use egui_node_graph::{Graph, GraphEditorState, UserResponseTrait};
use nodes::data::EditorNodeData;
use nodes::EditorNode;
use nodes::NodeType;

mod commands;
mod evaluator;
pub mod nodes;

/// The response type is used to encode side-effects produced when drawing a
/// node in the graph. Most side-effects (creating new nodes, deleting existing
/// nodes, handling connections...) are already handled by the library, but this
/// mechanism allows creating additional side effects from user code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorGraphResponse {}

/// The graph 'global' state. This state struct is passed around to the node and
/// parameter drawing callbacks. The contents of this struct are entirely up to
/// the user. For this example, we use it to keep track of the 'active' node.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EditorGraphState {}

impl UserResponseTrait for EditorGraphResponse {}

pub type EditorGraph = Graph<EditorNodeData, EDataType, EValue>;
pub type EditorState =
    GraphEditorState<EditorNodeData, EDataType, EValue, NodeType, EditorGraphState>;

pub fn evaluate_graph(graph: &EditorGraph) -> anyhow::Result<String> {
    let mut cache = Default::default();
    let mut commands = vec![];

    for (id, node) in &graph.nodes {
        if node.user_data.template.has_side_effects() {
            evaluate_node(graph, &mut cache, &mut commands, id)?
        }
    }
    let mut texts = vec![];
    for cmd in commands {
        match cmd {
            Command::Println(line) => {
                texts.push(line);
            }
        }
    }

    Ok(texts.join("\n"))
}

// #[derive(Default, Serialize, Deserialize)]
// pub struct NodeGraphExample {
//     // The `GraphEditorState` is the top-level object. You "register" all your
//     // custom types by specifying it as its generic parameters.
//     state: EditorState,
//
//     user_state: EditorGraphState,
// }
//
// const PERSISTENCE_KEY: &str = "egui_node_graph";
//
// impl NodeGraphExample {
//     /// If the persistence feature is enabled, Called once before the first frame.
//     /// Load previous app state (if any).
//     pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
//         let state = cc
//             .storage
//             .and_then(|storage| eframe::get_value(storage, PERSISTENCE_KEY))
//             .unwrap_or_default();
//         Self {
//             state,
//             user_state: EditorGraphState::default(),
//         }
//     }
// }
//
// impl eframe::App for NodeGraphExample {
//     /// Called each time the UI needs repainting, which may be many times per second.
//     /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//         egui::TopBottomPanel::top("top").show(ctx, |ui| {
//             egui::menu::bar(ui, |ui| {
//                 egui::widgets::global_dark_light_mode_switch(ui);
//             });
//         });
//         let graph_response = egui::CentralPanel::default()
//             .show(ctx, |ui| {
//                 self.state.draw_graph_editor(
//                     ui,
//                     AllEditorNodeTypes,
//                     &mut self.user_state,
//                     Vec::default(),
//                 )
//             })
//             .inner;
//         for _node_response in graph_response.node_responses {
//             // Here, we ignore all other graph events. But you may find
//             // some use for them. For example, by playing a sound when a new
//             // connection is created
//             // if let NodeResponse::User(user_event) = node_response {
//             //     match user_event {
//             //         MyResponse::SetActiveNode(node) => self.user_state.active_node = Some(node),
//             //         MyResponse::ClearActiveNode => self.user_state.active_node = None,
//             //     }
//             // }
//         }
//
//         let text = match evaluate_graph(&self.state.graph) {
//             Ok(text) => text,
//             Err(err) => format!("Execution error: {err}"),
//         };
//
//         ctx.debug_painter().text(
//             egui::pos2(10.0, 35.0),
//             egui::Align2::LEFT_TOP,
//             text,
//             TextStyle::Button.resolve(&ctx.style()),
//             egui::Color32::WHITE,
//         );
//     }
//
//     /// If the persistence function is enabled,
//     /// Called by the frame work to save state before shutdown.
//     fn save(&mut self, storage: &mut dyn eframe::Storage) {
//         eframe::set_value(storage, PERSISTENCE_KEY, &self.state);
//     }
// }
