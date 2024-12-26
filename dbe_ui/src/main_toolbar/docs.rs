use crate::DbeApp;
use dbe_backend::etype::eobject::EObject;
use dbe_backend::graph::node::{get_node_factory, NodeFactory};
use dbe_backend::project::docs::{DocsDescription, NodeDocs, TypeDocs};
use dbe_backend::registry::{EObjectType, ETypesRegistry};
use dbe_backend::value::id::ETypeId;
use egui::{RichText, Ui, Widget, WidgetText};
use egui_commonmark::CommonMarkCache;
use egui_hooks::UseHookExt;
use inline_tweak::tweak;
use std::ops::{Deref, DerefMut};
use strum::EnumIs;
use ustr::Ustr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, EnumIs)]
pub enum SelectedDocs {
    #[default]
    None,
    Node(String),
    Type(ETypeId),
}

pub fn docs_tab(ui: &mut Ui, app: &mut DbeApp) {
    let mut selection = ui.use_state(|| SelectedDocs::None, ()).into_var();
    ui.horizontal(|ui| {
        ui.label("Documentation");
        if ui
            .add_enabled(!selection.is_none(), egui::Button::new("Back"))
            .clicked()
        {
            *selection = SelectedDocs::None;
        }
    });

    let Some(project) = &app.project else {
        return;
    };
    egui::ScrollArea::vertical().show(ui, |ui| match selection.deref_mut() {
        SelectedDocs::None => {
            ui.collapsing("Nodes", |ui| {
                for (node_id, docs) in &project.docs.nodes {
                    let Some(factory) = get_node_factory(&Ustr::from(node_id)) else {
                        continue;
                    };
                    if ui.button(&docs.title).clicked() {
                        *selection = SelectedDocs::Node(node_id.clone());
                    }
                }
            });

            ui.collapsing("Types", |ui| {
                for (type_id, _) in &project.docs.types {
                    let Some(ty) = project.registry.get_object(type_id) else {
                        continue;
                    };
                    if ui.button(ty.title(&project.registry)).clicked() {
                        *selection = SelectedDocs::Type(*type_id);
                    }
                }
            });
        }
        SelectedDocs::Node(name) => {
            if let (Some(docs), Some(node)) = (
                project.docs.nodes.get(name),
                get_node_factory(&Ustr::from(name)),
            ) {
                node_docs(ui, node.deref(), docs);
            } else {
                *selection = SelectedDocs::None;
            }
        }
        SelectedDocs::Type(ty) => {
            if let (Some(docs), Some(ty)) =
                (project.docs.types.get(ty), project.registry.get_object(ty))
            {
                type_docs(ui, &project.registry, ty, docs);
            } else {
                *selection = SelectedDocs::None;
            }
        }
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
