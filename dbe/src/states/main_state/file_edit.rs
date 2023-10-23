use crate::dbe_files::EditorItem;
use crate::graph::nodes::AllEditorNodeTypes;
use crate::graph::EditorGraphState;
use camino::Utf8Path;
use egui::Ui;

use crate::states::main_state::TabHandler;

pub(super) fn show_file_edit(state: &mut TabHandler, ui: &mut Ui, path: &Utf8Path) {
    let Some(file) = state.0.state.fs.content_mut(path) else {
        ui.label(format!("Item with path {path} is not found"));
        return;
    };

    let EditorItem::Value(file) = file else {
        ui.label(format!("Item at {path} is not editable"));
        return;
    };
    let reg = &state.0.state.registry;

    let mut user_state = EditorGraphState {
        registry: reg.clone(),
    };

    let _ =
        file.graph
            .draw_graph_editor(ui, AllEditorNodeTypes(reg.clone()), &mut user_state, vec![]);
    // TODO: handle commands

    // match edited_value {
    //     EValue::Struct { ident, fields } => draw_struct(ui, &state.0.state.registry, ident, fields),
    //     _ => {
    //         ui.label("Only structs can be edited");
    //     }
    // }
    // draw_evalue(edited_value, ui, path.as_str(), &state.0.state.registry)
}
