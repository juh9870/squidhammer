use crate::graph::{evaluate_graph, EditorGraph, MyEditorState, MyGraphState};
use crate::nodes::AllMyNodeTemplates;
use eframe::egui::{self, TextStyle};
use rust_i18n::i18n;
use serde_derive::{Deserialize, Serialize};

mod commands;
mod evaluator;
mod graph;
mod nodes;
mod value;

i18n!();

// ========= First, define your user data types =============

// =========== Then, you need to implement some traits ============

#[derive(Default, Serialize, Deserialize)]
pub struct NodeGraphExample {
    // The `GraphEditorState` is the top-level object. You "register" all your
    // custom types by specifying it as its generic parameters.
    state: MyEditorState,

    user_state: MyGraphState,
}

const PERSISTENCE_KEY: &str = "egui_node_graph";

impl NodeGraphExample {
    /// If the persistence feature is enabled, Called once before the first frame.
    /// Load previous app state (if any).
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, PERSISTENCE_KEY))
            .unwrap_or_default();
        Self {
            state,
            user_state: MyGraphState::default(),
        }
    }
}

impl eframe::App for NodeGraphExample {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });
        let graph_response = egui::CentralPanel::default()
            .show(ctx, |ui| {
                self.state.draw_graph_editor(
                    ui,
                    AllMyNodeTemplates,
                    &mut self.user_state,
                    Vec::default(),
                )
            })
            .inner;
        for node_response in graph_response.node_responses {
            // Here, we ignore all other graph events. But you may find
            // some use for them. For example, by playing a sound when a new
            // connection is created
            // if let NodeResponse::User(user_event) = node_response {
            //     match user_event {
            //         MyResponse::SetActiveNode(node) => self.user_state.active_node = Some(node),
            //         MyResponse::ClearActiveNode => self.user_state.active_node = None,
            //     }
            // }
        }

        let text = match evaluate_graph(&self.state.graph) {
            Ok(text) => text,
            Err(err) => format!("Execution error: {err}"),
        };

        ctx.debug_painter().text(
            egui::pos2(10.0, 35.0),
            egui::Align2::LEFT_TOP,
            text,
            TextStyle::Button.resolve(&ctx.style()),
            egui::Color32::WHITE,
        );
    }

    /// If the persistence function is enabled,
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, PERSISTENCE_KEY, &self.state);
    }
}
