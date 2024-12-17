use dbe_backend::graph::inputs::{GraphInput, GraphOutput};
use dbe_backend::graph::Graph;
use egui::{ScrollArea, Ui};
use list_edit::list_editor;
use uuid::Uuid;

pub fn edit_inputs_outputs(ui: &mut Ui, graph: &mut Graph) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.label("Inputs");
        let inputs = graph.inputs_mut();
        list_editor::<GraphInput, _>(ui.id().with("inputs"))
            .new_item(|i| GraphInput {
                ty: None,
                id: Uuid::new_v4(),
                name: format!("input {}", i),
            })
            .show_custom(
                ui,
                inputs,
                |items, i| {
                    items.remove(i);
                },
                |items, item| {
                    items.push(item);
                },
                |ui, _, item| {
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut item.name);
                    });
                },
            );

        ui.label("Outputs");
        let outputs = graph.outputs_mut();
        list_editor::<GraphOutput, _>(ui.id().with("outputs"))
            .new_item(|i| GraphOutput {
                ty: None,
                id: Uuid::new_v4(),
                name: format!("output {}", i),
            })
            .show_custom(
                ui,
                outputs,
                |items, i| {
                    items.remove(i);
                },
                |items, item| {
                    items.push(item);
                },
                |ui, _, item| {
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut item.name);
                    });
                },
            );
    });
}
