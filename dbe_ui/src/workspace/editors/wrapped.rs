use crate::workspace::editors::utils::{unsupported, EditorSize};
use crate::workspace::editors::{DynProps, Editor, EditorContext, EditorResponse};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::etype::econst::ETypeConst;
use dbe_backend::etype::eitem::EItemInfo;
use dbe_backend::etype::EDataType;
use dbe_backend::registry::ETypesRegistry;
use dbe_backend::value::EValue;
use egui::Ui;
use ustr::Ustr;
use utils::map::HashMap;

/// Editor that wraps another editor for editing a single-field struct.
#[derive(Debug)]
pub struct WrappedEditor<T: Editor> {
    editor: T,
    field: Ustr,
}

impl<T: Editor> WrappedEditor<T> {
    pub fn new(editor: T, field: Ustr) -> Self {
        Self { editor, field }
    }
}

impl<T: Editor> Editor for WrappedEditor<T> {
    fn props(
        &self,
        reg: &ETypesRegistry,
        item: Option<&EItemInfo>,
        object_props: DynProps,
    ) -> miette::Result<DynProps> {
        let Some(item) = item else {
            return self.editor.props(reg, item, object_props);
        };

        let EDataType::Object { ident } = item.ty() else {
            return Ok(None);
        };

        let Some(data) = reg.get_struct(&ident) else {
            return Ok(None);
        };

        if data.fields.len() != 1 {
            return Ok(None);
        }

        let field = &data.fields[0];
        if field.name != self.field {
            return Ok(None);
        }

        self.editor.props(reg, Some(&field.ty), object_props)
    }

    fn size(&self, props: &DynProps) -> EditorSize {
        self.editor.size(props)
    }

    fn edit(
        &self,
        ui: &mut Ui,
        ctx: EditorContext,
        diagnostics: DiagnosticContextRef,
        field_name: &str,
        value: &mut EValue,
        props: &DynProps,
    ) -> EditorResponse {
        let EValue::Struct { fields, ident: _ } = value else {
            unsupported!(ui, field_name, value, self);
        };

        if fields.len() != 1 {
            unsupported!(ui, field_name, value, self);
        }

        let Some(field) = fields.get_mut(&self.field) else {
            unsupported!(ui, field_name, value, self);
        };

        self.editor
            .edit(ui, ctx, diagnostics, field_name, field, props)
    }
}
