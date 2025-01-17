use crate::workspace::editors::utils::{
    labeled_collapsing_header, unsupported, EditorResultExt, EditorSize,
};
use crate::workspace::editors::{editor_for_type, DynProps, Editor, EditorContext, EditorResponse};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::project::docs::DocsRef;
use dbe_backend::value::EValue;
use egui::{Button, Ui};
use egui_hooks::UseHookExt;
use miette::miette;
use utils::map::HashSet;

#[derive(Debug)]
pub struct MapEditor;

impl Editor for MapEditor {
    fn size(&self, _props: &DynProps) -> EditorSize {
        EditorSize::Block
    }

    fn edit(
        &self,
        ui: &mut Ui,
        mut ctx: EditorContext,
        mut diagnostics: DiagnosticContextRef,
        field_name: &str,
        value: &mut EValue,
        _props: &DynProps,
    ) -> EditorResponse {
        let EValue::Map { values, id } = value else {
            unsupported!(ui, field_name, value, self);
        };

        let mut changed = false;
        let docs_ctx = ctx.replace_docs_ref(DocsRef::None);
        // let res = labeled_field(ui, field_name, ctx, |ui| {
        //     ui.toggle_value(value, if *value { "⏹ True" } else { "☐ False" })
        // });
        ctx.registry
            .get_map(id)
            .ok_or_else(|| miette!("!!INTERNAL ERROR!! unknown map `{}`", id))
            .then_draw(ui, |ui, map_data| {
                labeled_collapsing_header(
                    ui,
                    field_name,
                    docs_ctx,
                    values.len() < 20,
                    true,
                    |ui| {
                        let value_ty = map_data.value_type;
                        let key_ty = map_data.key_type;
                        let value_editor = editor_for_type(ctx.registry, &value_ty);
                        let key_editor = editor_for_type(ctx.registry, &key_ty);

                        let used_keys = values.keys().cloned().collect::<HashSet<_>>();
                        let mut moved_key = None::<(EValue, EValue)>;
                        let mut removed_key = None::<EValue>;

                        egui::Grid::new(field_name).striped(true).show(ui, |ui| {
                            for (idx, (key, val)) in values.iter_mut().enumerate() {
                                let key_str = key.to_string();

                                let mut d = diagnostics.enter_map_key(key_str.clone());

                                ui.push_id((&key, "key"), |ui| {
                                    let mut edited_key = ui
                                        .use_state(|| None::<EditedKey>, key_str.clone())
                                        .into_var();

                                    let mut label_shown = false;
                                    let mut key_editing_done = false;
                                    if let Some(edited) =
                                        edited_key.as_mut().filter(|e| e.index == idx)
                                    {
                                        // edited key doesn't match the actual key, cancel the edit
                                        if &edited.old_key != key {
                                            *edited_key = None;
                                        } else {
                                            ui.horizontal(|ui| {
                                                if ui.button("Cancel").clicked() {
                                                    key_editing_done = true;
                                                }
                                                let key_in_use =
                                                    used_keys.contains(&edited.new_key);
                                                let key_changed = edited.new_key != edited.old_key;
                                                if ui
                                                    .add_enabled(
                                                        !key_in_use && key_changed,
                                                        Button::new(if !key_changed {
                                                            "No change"
                                                        } else if key_in_use {
                                                            "Key already in use"
                                                        } else {
                                                            "Apply"
                                                        }),
                                                    )
                                                    .clicked()
                                                    && moved_key.is_none()
                                                {
                                                    moved_key =
                                                        Some((key.clone(), edited.new_key.clone()));
                                                    key_editing_done = true;
                                                }
                                                key_editor.show(
                                                    ui,
                                                    ctx.copy_with_docs(DocsRef::None),
                                                    d.enter_inline(),
                                                    "",
                                                    &mut edited.new_key,
                                                );
                                            });
                                            label_shown = true;
                                        }
                                    }

                                    if key_editing_done {
                                        *edited_key = None;
                                    }

                                    if !label_shown {
                                        let label = ui.selectable_label(false, &key_str);
                                        if label.clicked() {
                                            *edited_key = Some(EditedKey {
                                                index: idx,
                                                old_key: key.clone(),
                                                new_key: key.clone(),
                                            });
                                        };
                                        label.context_menu(|ui| {
                                            if ui.button("Remove").clicked() {
                                                removed_key = Some(key.clone());
                                                ui.close_menu();
                                            }
                                        });
                                    }
                                });

                                ui.push_id((&key, "value"), |ui| {
                                    if value_editor
                                        .show(
                                            ui,
                                            ctx.copy_with_docs(DocsRef::None),
                                            d.enter_inline(),
                                            "",
                                            val,
                                        )
                                        .changed
                                    {
                                        changed = true;
                                    }
                                });

                                ui.end_row();
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.push_id("add-new_key", |ui| {
                                let mut new_key = ui
                                    .use_state(
                                        || key_ty.default_value(ctx.registry).into_owned(),
                                        (),
                                    )
                                    .into_var();
                                let already_present = values.contains_key(&new_key);
                                if ui
                                    .add_enabled(
                                        !already_present,
                                        Button::new(if already_present {
                                            "Key already present"
                                        } else {
                                            "Add new entry"
                                        }),
                                    )
                                    .clicked()
                                {
                                    values.insert(
                                        new_key.clone(),
                                        value_ty.default_value(ctx.registry).into_owned(),
                                    );
                                    *new_key = key_ty.default_value(ctx.registry).into_owned();
                                    changed = true;
                                }

                                key_editor.show(
                                    ui,
                                    ctx.copy_with_docs(DocsRef::None),
                                    diagnostics.enter_inline(),
                                    "",
                                    &mut new_key,
                                );
                            });
                        });

                        if let Some((old_key, new_key)) = moved_key {
                            let value = values.remove(&old_key).unwrap();
                            values.insert(new_key, value);
                            changed = true;
                        }

                        if let Some(key) = removed_key {
                            values.remove(&key);
                            changed = true;
                        }
                    },
                );
            });

        EditorResponse::new(changed)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct EditedKey {
    index: usize,
    old_key: EValue,
    new_key: EValue,
}
