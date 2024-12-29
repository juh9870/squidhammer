use crate::workspace::editors::utils::{labeled_error, labeled_field, EditorSize};
use crate::workspace::editors::{
    cast_props, DynProps, Editor, EditorContext, EditorProps, EditorResponse,
};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::etype::econst::ETypeConst;
use dbe_backend::etype::eitem::EItemInfo;
use dbe_backend::etype::EDataType;
use dbe_backend::registry::ETypesRegistry;
use dbe_backend::value::EValue;
use egui::Ui;
use miette::{bail, miette};

#[derive(Debug)]
pub struct ConstEditor;

impl Editor for ConstEditor {
    fn props(&self, _reg: &ETypesRegistry, item: Option<&EItemInfo>) -> miette::Result<DynProps> {
        if let Some(ty) = item.map(|i| i.ty()) {
            let EDataType::Const { value } = ty else {
                bail!("unsupported item. Expected const")
            };

            Ok(ConstEditorProps { item: Some(value) }.pack())
        } else {
            Ok(ConstEditorProps { item: None }.pack())
        }
    }

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
        props: &DynProps,
    ) -> EditorResponse {
        let props = cast_props::<ConstEditorProps>(props);
        if let Some(item) = props.item {
            let const_value = item.default_value();
            if value != &const_value {
                labeled_error(ui, field_name, miette!("{}", ("dbe.editor.bad_const")))
            }
        }

        labeled_field(ui, field_name, ctx, |ui| ui.label(value.to_string()));

        EditorResponse::unchanged()
    }
}

#[derive(Debug, Clone)]
struct ConstEditorProps {
    item: Option<ETypeConst>,
}

impl EditorProps for ConstEditorProps {}
