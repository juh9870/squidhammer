use crate::workspace::editors::utils::{labeled_field, prop, unsupported, EditorSize};
use crate::workspace::editors::{cast_props, DynProps, Editor, EditorProps, EditorResponse};
use dbe2::etype::eitem::EItemInfo;
use dbe2::registry::ETypesRegistry;
use dbe2::value::EValue;
use egui::{TextEdit, Ui};

#[derive(Debug, Clone)]
pub struct StringEditor;
impl Editor for StringEditor {
    fn props(&self, _reg: &ETypesRegistry, item: Option<&EItemInfo>) -> miette::Result<DynProps> {
        let props = item.map(|i| i.extra_properties());
        let multiline = prop(props, "multiline", false)?;

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
        _reg: &ETypesRegistry,
        field_name: &str,
        value: &mut EValue,
        props: &DynProps,
    ) -> EditorResponse {
        let Ok(value) = value.try_as_string_mut() else {
            unsupported!(ui, field_name, value, self);
        };
        let props = cast_props::<StringProps>(props);
        let res = labeled_field(ui, field_name, |ui| {
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
