use camino::Utf8Path;
use egui::Ui;

use crate::states::main_state::TabHandler;
use crate::value::draw::draw_struct;
use crate::value::EValue;

pub(super) fn show_file_edit(
    state: &mut TabHandler,
    ui: &mut Ui,
    _path: &Utf8Path,
    edited_value: &mut EValue,
) {
    match edited_value {
        EValue::Struct { ident, fields } => draw_struct(ui, &state.0.state.registry, ident, fields),
        _ => {
            ui.label("Only structs can be edited");
        }
    }
    // draw_evalue(edited_value, ui, path.as_str(), &state.0.state.registry)
}
