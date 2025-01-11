use dbe_backend::graph::inputs::{GraphInput, GraphIoData, GraphOutput};
use dbe_backend::graph::Graph;
use egui::{ScrollArea, Ui};
use list_edit::list_editor;
use smallvec::SmallVec;
use std::hash::Hash;
use uuid::Uuid;

pub fn edit_io<IO: GraphIoData, const N: usize>(
    ui: &mut Ui,
    values: &mut SmallVec<[IO; N]>,
    id_salt: impl Hash,
    new_item: impl Fn(usize) -> IO,
) {
    list_editor::<IO, _>(ui.id().with(id_salt))
        .new_item(new_item)
        .show_custom(
            ui,
            values,
            |items, i| {
                items.remove(i);
            },
            |items, item| {
                items.push(item);
            },
            |ui, _, item| {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(item.name_mut());
                });
            },
        );
}
pub fn edit_inputs_outputs(ui: &mut Ui, graph: &mut Graph) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.label("Inputs");
        let inputs = graph.inputs_mut();
        edit_io(ui, inputs, "inputs", |i| GraphInput {
            ty: None,
            id: Uuid::new_v4(),
            name: format!("input {}", i),
        });

        ui.label("Outputs");
        let outputs = graph.outputs_mut();
        edit_io(ui, outputs, "outputs", |i| GraphOutput {
            ty: None,
            id: Uuid::new_v4(),
            name: format!("output {}", i),
        });
    });
}
