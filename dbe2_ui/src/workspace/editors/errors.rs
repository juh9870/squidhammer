use crate::workspace::editors::utils::{labeled_error, EditorSize};
use crate::workspace::editors::{cast_props, DynProps, Editor, EditorProps};
use dbe2::registry::ETypesRegistry;
use dbe2::value::EValue;
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
        _reg: &ETypesRegistry,
        field_name: &str,
        _value: &mut EValue,
        props: &DynProps,
    ) {
        let props = cast_props::<ErrorProps>(props);
        labeled_error(ui, field_name, miette!("{}", props.0))
    }
}

#[derive(Debug, Clone)]
pub struct ErrorProps(pub String);

impl EditorProps for ErrorProps {}
