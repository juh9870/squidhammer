use crate::workspace::editors::utils::{
    labeled_field, prop, unsupported, EditorResultExt, EditorSize,
};
use crate::workspace::editors::{cast_props, editor_for_item, DynProps, Editor, EditorProps};
use dbe2::etype::eitem::EItemInfo;
use dbe2::registry::ETypesRegistry;
use dbe2::value::EValue;
use egui::Ui;
use itertools::Itertools;
use miette::miette;

#[derive(Debug)]
pub struct StructEditor;

impl Editor for StructEditor {
    fn props(&self, _reg: &ETypesRegistry, item: Option<&EItemInfo>) -> miette::Result<DynProps> {
        if prop(item.map(|i| i.extra_properties()), "inline", false)? {
            Ok(StructProps { inline: true }.pack())
        } else {
            Ok(StructProps { inline: false }.pack())
        }
    }

    fn size(&self, _props: &DynProps) -> EditorSize {
        EditorSize::Block
    }

    fn edit(
        &self,
        ui: &mut Ui,
        reg: &ETypesRegistry,
        field_name: &str,
        value: &mut EValue,
        props: &DynProps,
    ) {
        let EValue::Struct { fields, ident } = value else {
            unsupported!(ui, field_name, value, self);
        };

        let props = cast_props::<StructProps>(props);

        reg.get_struct(ident)
            .ok_or_else(|| miette!("unknown struct `{}`", ident))
            .then_draw(ui, |ui, data| {
                let items = data
                    .fields
                    .iter()
                    .map(|f| (f, editor_for_item(reg, &f.ty)))
                    .collect_vec();

                let inline =
                    items.len() <= 3 && items.iter().all(|f| f.1.size() <= EditorSize::Inline);

                let draw_fields = |ui: &mut Ui| {
                    for (field, editor) in items {
                        fields
                            .get_mut(&field.name)
                            .ok_or_else(|| miette!("field `{}` is missing", field.name))
                            .then_draw(ui, |ui, value| {
                                editor.show(ui, reg, field.name.as_ref(), value);
                            });
                    }
                };

                if inline {
                    ui.horizontal(|ui| {
                        labeled_field(ui, field_name, draw_fields);
                    });
                } else if props.inline {
                    draw_fields(ui);
                } else {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            if !field_name.is_empty() {
                                ui.label(field_name);
                            }
                            draw_fields(ui);
                        })
                    });
                }
            });
    }
}

#[derive(Debug, Clone)]
struct StructProps {
    inline: bool,
}

impl EditorProps for StructProps {}
