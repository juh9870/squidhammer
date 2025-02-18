use crate::ui_props::PROP_FIELD_MULTILINE;
use crate::workspace::editors::utils::{labeled_field, unsupported, EditorSize};
use crate::workspace::editors::{
    cast_props, DynProps, Editor, EditorContext, EditorProps, EditorResponse,
};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::etype::eitem::EItemInfo;
use dbe_backend::registry::ETypesRegistry;
use dbe_backend::value::EValue;
use egui::{ScrollArea, TextEdit, Ui};
use egui_hooks::UseHookExt;
use inline_tweak::tweak;

#[derive(Debug, Clone)]
pub struct StringEditor;
impl Editor for StringEditor {
    fn props(
        &self,
        _reg: &ETypesRegistry,
        item: Option<&EItemInfo>,
        _object_props: DynProps,
    ) -> miette::Result<DynProps> {
        let props = item.map(EItemInfo::extra_properties);
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
        let scale = ui.ctx().style().spacing.combo_width / ui.style().spacing.combo_width;
        let max_size = tweak!(200.0) / scale;
        let use_scrollbar = ui.use_state(|| false, ());
        let edit_id = ui.id().with("text_editor");
        let res = labeled_field(ui, field_name, ctx, |ui| {
            let edit = if props.multiline {
                TextEdit::multiline(value)
            } else {
                TextEdit::singleline(value)
            }
            .clip_text(false)
            .desired_width(if props.multiline { max_size } else { 0.0 })
            .desired_rows(1)
            .margin(ui.spacing().item_spacing)
            .id(edit_id);
            if *use_scrollbar {
                ScrollArea::horizontal()
                    .max_width(max_size)
                    .show(ui, |ui| edit.show(ui))
                    .inner
            } else {
                edit.show(ui)
            }
        });

        if res.inner.response.changed() {
            use_scrollbar.set_next(res.inner.galley.rect.width() > max_size);
        }

        EditorResponse::new(res.inner.response.changed())
    }
}

#[derive(Debug, Clone)]
struct StringProps {
    multiline: bool,
}

impl EditorProps for StringProps {}
