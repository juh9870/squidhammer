use crate::workspace::editors::utils::{labeled_error, labeled_field, EditorSize};
use crate::workspace::editors::{cast_props, DynProps, Editor, EditorProps, EditorResponse};
use dbe2::etype::econst::ETypeConst;
use dbe2::etype::eitem::EItemInfo;
use dbe2::etype::EDataType;
use dbe2::registry::ETypesRegistry;
use dbe2::value::EValue;
use egui::Ui;
use miette::{bail, miette};

#[derive(Debug)]
pub struct ConstEditor;

impl Editor for ConstEditor {
    fn props(&self, _reg: &ETypesRegistry, item: Option<&EItemInfo>) -> miette::Result<DynProps> {
        let Some(ty) = item.map(|i| i.ty()) else {
            bail!("!!INTERNAL ERROR!! const editor can't be used without providing EItemType");
        };
        let EDataType::Const { value } = ty else {
            bail!("unsupported item. Expected const")
        };

        Ok(ConstEditorProps { item: value }.pack())
    }

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
        let props = cast_props::<ConstEditorProps>(props);
        let const_value = props.item.default_value();
        if value != &const_value {
            labeled_error(ui, field_name, miette!("{}", ("dbe.editor.bad_const")))
        }

        labeled_field(ui, field_name, |ui| ui.label(value.to_string()));

        EditorResponse::unchanged()
    }
}

#[derive(Debug, Clone)]
struct ConstEditorProps {
    item: ETypeConst,
}

impl EditorProps for ConstEditorProps {}
