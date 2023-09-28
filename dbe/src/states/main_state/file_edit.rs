use camino::Utf8Path;
use egui::Ui;

use crate::states::main_state::TabHandler;
use crate::value::draw::draw_evalue;
use crate::value::EValue;

pub(super) fn show_file_edit(
    state: &mut TabHandler,
    ui: &mut Ui,
    path: &Utf8Path,
    edited_value: &mut EValue,
) {
    draw_evalue(edited_value, ui, path.as_str(), &state.0.state.registry)
}
