use crate::workspace::editors::{editor_for_value, EditorContext, EditorResponse};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::value::EValue;
use egui::Ui;
use ustr::Ustr;

pub fn quick_edit_evalue(
    ui: &mut Ui,
    ctx: EditorContext,
    diagnostics: DiagnosticContextRef,
    field_name: &Ustr,
    value: &mut EValue,
) -> EditorResponse {
    let editor = editor_for_value(ctx.registry, value);
    editor.show(ui, ctx, diagnostics, field_name, value)
}
