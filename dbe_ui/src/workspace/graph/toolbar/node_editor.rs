use crate::main_toolbar::docs::docs_label;
use crate::workspace::graph::toolbar::edit_inputs::edit_io;
use dbe_backend::graph::region::RegionVariable;
use dbe_backend::graph::Graph;
use dbe_backend::project::docs::{Docs, DocsRef};
use dbe_backend::registry::ETypesRegistry;
use egui::{CollapsingHeader, Color32, ScrollArea, Ui};
use egui_snarl::NodeId;
use uuid::Uuid;

pub fn edit_node_properties(
    ui: &mut Ui,
    registry: &ETypesRegistry,
    docs: &Docs,
    graph: &mut Graph,
    selected_nodes: &[NodeId],
) {
    ScrollArea::vertical().show(ui, |ui| {
        CollapsingHeader::new("Region")
            .default_open(true)
            .show(ui, |ui| {
                edit_node_region(ui, registry, docs, graph, selected_nodes);
            });
    });
}

pub fn edit_node_region(
    ui: &mut Ui,
    registry: &ETypesRegistry,
    docs: &Docs,
    graph: &mut Graph,
    selected_nodes: &[NodeId],
) {
    let Some(node) = exactly_one_node(ui, selected_nodes) else {
        return;
    };

    let node = &graph.snarl()[node];
    let Some(region) = node.region_source().or_else(|| node.region_end()) else {
        ui.label("Selected node must be start or end of a region");
        return;
    };

    let reg = graph
        .regions_mut()
        .get_mut(&region)
        .expect("Region should exist");
    ui.horizontal(|ui| {
        docs_label(ui, "Region Color", docs, registry, DocsRef::Custom("Color of the region as visible in graph editor. If not set, a random color will be used".into()));
        let mut checked = reg.color.is_some();
        ui.checkbox(&mut checked, "");
        if checked {
            let cur_color = reg.color();
            let color = reg.color.get_or_insert(cur_color);
            rgb_edit(ui, color);
        } else {
            reg.color = None;
            let mut cur_color = reg.color();
            ui.add_enabled_ui(false, |ui| {
                ui.color_edit_button_srgba(&mut cur_color);
            });
        }
    });

    ui.label("Variables");
    edit_io(ui, &mut reg.variables, "variables", |i| RegionVariable {
        ty: None,
        id: Uuid::new_v4(),
        name: format!("value {}", i),
    });
}

fn at_least_one_node(ui: &mut Ui, selected_nodes: &[NodeId]) -> bool {
    if selected_nodes.is_empty() {
        ui.label("No node selected");
        ui.label("SHIFT+Click to select nodes");
        return false;
    }
    true
}

fn exactly_one_node(ui: &mut Ui, selected_nodes: &[NodeId]) -> Option<NodeId> {
    if !at_least_one_node(ui, selected_nodes) {
        return None;
    } else if selected_nodes.len() > 1 {
        ui.label("A single node must be selected");
        ui.label("CTRL+Click to deselect nodes");
        return None;
    }

    Some(selected_nodes[0])
}

fn rgb_edit(ui: &mut Ui, color: &mut Color32) {
    let rgba = color.to_array();
    let mut rgb = [rgba[0], rgba[1], rgba[2]];
    ui.color_edit_button_srgb(&mut rgb);
    *color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
}
