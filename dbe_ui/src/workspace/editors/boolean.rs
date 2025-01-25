use crate::widgets::toggle_button::toggle_button;
use crate::workspace::editors::utils::{labeled_field, unsupported, EditorSize};
use crate::workspace::editors::{DynProps, Editor, EditorContext, EditorResponse};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::value::EValue;
use egui::Ui;

#[derive(Debug)]
pub struct BooleanEditor;

impl Editor for BooleanEditor {
    fn size(&self, _props: &DynProps) -> EditorSize {
        EditorSize::Inline
    }

    fn edit(
        &self,
        ui: &mut Ui,
        ctx: EditorContext,
        _diagnostics: DiagnosticContextRef,
        field_name: &str,
        value: &mut EValue,
        _props: &DynProps,
    ) -> EditorResponse {
        let Ok(value) = value.try_as_boolean_mut() else {
            unsupported!(ui, field_name, value, self);
        };
        let res = labeled_field(ui, field_name, ctx, |ui| toggle_button(ui, value));

        EditorResponse::new(res.inner.changed())
    }
}
