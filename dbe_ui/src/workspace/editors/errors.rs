use crate::workspace::editors::utils::{labeled_error, EditorSize};
use crate::workspace::editors::{
    cast_props, DynProps, Editor, EditorContext, EditorProps, EditorResponse,
};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::value::EValue;
use egui::Ui;
use miette::miette;

#[derive(Debug, Clone)]
pub struct ErrorEditor;

impl Editor for ErrorEditor {
    fn size(&self, _props: &DynProps) -> EditorSize {
        EditorSize::Inline
    }

    fn edit(
        &self,
        ui: &mut Ui,
        _ctx: EditorContext,
        _diagnostics: DiagnosticContextRef,
        field_name: &str,
        _value: &mut EValue,
        props: &DynProps,
    ) -> EditorResponse {
        let props = cast_props::<ErrorProps>(props);
        labeled_error(ui, field_name, miette!("{}", props.0));
        EditorResponse::unchanged()
    }
}

#[derive(Debug, Clone)]
pub struct ErrorProps(pub String);

impl EditorProps for ErrorProps {}
