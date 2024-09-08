use crate::workspace::editors::utils::{labeled_field, unsupported, EditorSize};
use crate::workspace::editors::{DynProps, Editor, EditorResponse};
use dbe2::registry::ETypesRegistry;
use dbe2::value::EValue;
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
        reg: &ETypesRegistry,
        field_name: &str,
        value: &mut EValue,
        props: &DynProps,
    ) -> EditorResponse {
        let Ok(value) = value.try_as_boolean_mut() else {
            unsupported!(ui, field_name, value, self);
        };
        let res = labeled_field(ui, field_name, |ui| {
            ui.toggle_value(value, if *value { "⏹ True" } else { "☐ False" })
        });

        EditorResponse::new(res.inner.changed())
    }
}
