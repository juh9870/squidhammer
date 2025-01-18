use crate::ui_props::PROP_FIELD_MULTILINE;
use crate::workspace::editors::utils::{labeled_field, unsupported, EditorSize};
use crate::workspace::editors::{
    cast_props, DynProps, Editor, EditorContext, EditorProps, EditorResponse,
};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::etype::econst::ETypeConst;
use dbe_backend::etype::eitem::EItemInfo;
use dbe_backend::registry::ETypesRegistry;
use dbe_backend::value::EValue;
use egui::{TextEdit, Ui};
use utils::map::HashMap;

#[derive(Debug, Clone)]
pub struct StringEditor;
impl Editor for StringEditor {
    fn props(
        &self,
        _reg: &ETypesRegistry,
        item: Option<&EItemInfo>,
        _object_props: DynProps,
    ) -> miette::Result<DynProps> {
        let props = item.map(|i| i.extra_properties());
        let multiline = props
            .and_then(|p| PROP_FIELD_MULTILINE.try_get(p))
            .unwrap_or(false);

        Ok(StringProps { multiline }.pack())
    }

    fn size(&self, props: &DynProps) -> EditorSize {
        let props = cast_props::<StringProps>(props);
        if props.multiline {
            EditorSize::Block
        } else {
            EditorSize::Inline
        }
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
        let Ok(value) = value.try_as_string_mut() else {
            unsupported!(ui, field_name, value, self);
        };
        let props = cast_props::<StringProps>(props);
        let res = labeled_field(ui, field_name, ctx, |ui| {
            if props.multiline {
                TextEdit::multiline(value)
            } else {
                TextEdit::singleline(value)
            }
            .clip_text(false)
            .desired_width(0.0)
            .margin(ui.spacing().item_spacing)
            .show(ui)
        });

        EditorResponse::new(res.inner.response.changed())
    }
}

#[derive(Debug, Clone)]
struct StringProps {
    multiline: bool,
}

impl EditorProps for StringProps {}
