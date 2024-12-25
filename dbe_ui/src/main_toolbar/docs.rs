use crate::DbeApp;
use dbe_backend::graph::node::{get_node_factory, NodeFactory};
use dbe_backend::project::docs::{DocsDescription, NodeDocs, NodeIODocs};
use egui::{RichText, Ui, Widget, WidgetText};
use egui_commonmark::CommonMarkCache;
use inline_tweak::tweak;
use std::ops::Deref;
use ustr::Ustr;

pub fn docs_tab(ui: &mut Ui, app: &mut DbeApp) {
    ui.label("Documentation");
    let Some(project) = &app.project else {
        return;
    };
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.collapsing("Nodes", |ui| {
            for (node_id, docs) in &project.docs.nodes {
                let Some(factory) = get_node_factory(&Ustr::from(node_id)) else {
                    continue;
                };
                egui::CollapsingHeader::new(&docs.title)
                    .id_salt(node_id)
                    .show(ui, |ui| {
                        node_docs(ui, factory.deref(), docs);
                    });
            }
        });

        ui.collapsing("Types", |ui| {
            for (type_id, docs) in &project.docs.types {
                todo!("Show type docs");
            }
        })
    });
}

fn labeled_text(ui: &mut Ui, label: impl Into<WidgetText>, text: impl Into<WidgetText>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(label);
        ui.label(text);
    });
}

fn show_description(ui: &mut Ui, docs: &impl DocsDescription, md_cache: &mut CommonMarkCache) {
    ui.label(docs.description());

    if !docs.docs().is_empty() {
        ui.label("");
        egui_commonmark::CommonMarkViewer::new().show(ui, md_cache, docs.docs());
    }
}

pub fn node_docs(ui: &mut Ui, node: &dyn NodeFactory, docs: &NodeDocs) {
    egui::Label::new(RichText::new(&docs.title).heading())
        .ui(ui)
        .on_hover_text(format!("ID: {}", node.id()));

    if !node.categories().is_empty() {
        labeled_text(ui, "Categories: ", node.categories().join(", "));
    }

    ui.separator();

    let mut md_cache = CommonMarkCache::default();

    show_description(ui, docs, &mut md_cache);

    ui.style_mut().visuals.indent_has_left_vline = false;

    let mut docs_io = |ui: &mut Ui, docs: &NodeIODocs| {
        egui::CollapsingHeader::new(RichText::new(&docs.title).strong().size(tweak!(14.0)))
            .default_open(true)
            .show(ui, |ui| {
                show_description(ui, docs, &mut md_cache);
            });
    };

    if !docs.inputs.is_empty() {
        ui.label(RichText::new("Inputs").heading());
        ui.separator();
        for docs in &docs.inputs {
            docs_io(ui, docs)
        }
    }
    if !docs.outputs.is_empty() {
        ui.label(RichText::new("Outputs").heading());
        ui.separator();
        for docs in &docs.outputs {
            docs_io(ui, docs)
        }
    }
}
