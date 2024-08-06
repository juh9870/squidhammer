use crate::workspace::editors::utils::{labeled_field, unsupported, EditorResultExt, EditorSize};
use crate::workspace::editors::{editor_for_item, DynProps, Editor};
use dbe2::registry::ETypesRegistry;
use dbe2::value::EValue;
use egui::Ui;
use itertools::Itertools;
use miette::miette;

#[derive(Debug)]
pub struct StructEditor;

impl Editor for StructEditor {
    fn size(&self, _props: &DynProps) -> EditorSize {
        EditorSize::Block
    }

    fn edit(
        &self,
        ui: &mut Ui,
        reg: &ETypesRegistry,
        field_name: &str,
        value: &mut EValue,
        _props: &DynProps,
    ) {
        let EValue::Struct { fields, ident } = value else {
            unsupported!(ui, field_name, value, self);
        };

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
