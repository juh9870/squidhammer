use crate::DbeApp;
use dbe_backend::etype::eobject::EObject;
use dbe_backend::graph::node::{get_node_factory, NodeFactory};
use dbe_backend::project::docs::{DocsDescription, NodeDocs, TypeDocs};
use dbe_backend::registry::{EObjectType, ETypesRegistry};
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
                let Some(ty) = project.registry.get_object(type_id) else {
                    continue;
                };
                egui::CollapsingHeader::new(ty.title(&project.registry))
                    .id_salt(type_id)
                    .show(ui, |ui| {
                        type_docs(ui, &project.registry, ty, docs);
                    });
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

fn show_collapsing_description(
    ui: &mut Ui,
    docs: &impl DocsDescription,
    md_cache: &mut CommonMarkCache,
    title: &str,
) {
    egui::CollapsingHeader::new(RichText::new(title).strong().size(tweak!(14.0)))
        .default_open(true)
        .show(ui, |ui| {
            show_description(ui, docs, md_cache);
        });
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
    if !docs.inputs.is_empty() {
        ui.label(RichText::new("Inputs").heading());
        ui.separator();
        for docs in &docs.inputs {
            show_collapsing_description(ui, docs, &mut md_cache, &docs.title);
        }
    }
    if !docs.outputs.is_empty() {
        ui.label(RichText::new("Outputs").heading());
        ui.separator();
        for docs in &docs.outputs {
            show_collapsing_description(ui, docs, &mut md_cache, &docs.title);
        }
    }
}

pub fn type_docs(ui: &mut Ui, registry: &ETypesRegistry, ty: &EObjectType, docs: &TypeDocs) {
    egui::Label::new(RichText::new(ty.title(registry)).heading())
        .ui(ui)
        .on_hover_text(format!("ID: {}", ty.ident()));

    ui.separator();

    let mut md_cache = CommonMarkCache::default();

    show_description(ui, docs, &mut md_cache);

    ui.style_mut().visuals.indent_has_left_vline = false;
    if ty.as_enum().is_some() {
        ui.label(RichText::new("Variants").heading());
        ui.separator();
        for docs in &docs.variants {
            show_collapsing_description(ui, docs, &mut md_cache, &docs.id);
        }
    } else {
        ui.label(RichText::new("Fields").heading());
        ui.separator();
        for docs in &docs.fields {
            show_collapsing_description(ui, docs, &mut md_cache, &docs.id);
        }
    }
}
