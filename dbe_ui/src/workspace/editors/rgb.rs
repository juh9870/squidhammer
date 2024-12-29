use crate::main_toolbar::docs::DocsRef;
use crate::workspace::editors::utils::{
    ensure_field, get_values, labeled_field, set_values, unsupported, EditorResultExt, EditorSize,
};
use crate::workspace::editors::{DynProps, Editor, EditorContext, EditorResponse};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::value::{ENumber, EValue};
use egui::collapsing_header::CollapsingState;
use egui::{DragValue, Ui};

#[derive(Debug)]
pub struct RgbEditor {
    with_alpha: bool,
}

impl RgbEditor {
    pub fn new(with_alpha: bool) -> Self {
        Self { with_alpha }
    }
}

impl Editor for RgbEditor {
    fn size(&self, _props: &DynProps) -> EditorSize {
        EditorSize::Block
    }

    fn edit(
        &self,
        ui: &mut Ui,
        mut ctx: EditorContext,
        _diagnostics: DiagnosticContextRef,
        field_name: &str,
        value: &mut EValue,
        _props: &DynProps,
    ) -> EditorResponse {
        let field_names = ["r", "g", "b", if self.with_alpha { "a" } else { "" }];
        let EValue::Struct { fields, .. } = value else {
            unsupported!(ui, field_name, value, self);
        };

        let mut changed = false;
        CollapsingState::load_with_default_open(ui.ctx(), ui.id().with(field_name), false)
            .show_header(ui, |ui| {
                labeled_field(ui, field_name, ctx.replace_docs_ref(DocsRef::None), |ui| {
                    if self.with_alpha {
                        get_values::<f32, _, 4>(fields, ["r", "g", "b", "a"]).then_draw(
                            ui,
                            |ui, mut value| {
                                if ui.color_edit_button_rgba_unmultiplied(&mut value).changed() {
                                    changed = true;
                                };
                                set_values(
                                    fields,
                                    [
                                        ("r", value[0]),
                                        ("g", value[1]),
                                        ("b", value[2]),
                                        ("a", value[3]),
                                    ],
                                )
                            },
                        );
                    } else {
                        get_values::<f32, _, 3>(fields, ["r", "g", "b"]).then_draw(
                            ui,
                            |ui, mut value| {
                                if ui.color_edit_button_rgb(&mut value).changed() {
                                    changed = true;
                                };
                                set_values(
                                    fields,
                                    [("r", value[0]), ("g", value[1]), ("b", value[2])],
                                );
                            },
                        );
                    }
                });
            })
            .body(|ui| {
                ui.vertical(|ui| {
                    for name in field_names {
                        ensure_field(ui, fields, name, |ui, value: &mut ENumber| {
                            labeled_field(ui, name, ctx.copy_with_docs(DocsRef::None), |ui| {
                                if ui
                                    .add(DragValue::new(&mut value.0).range(0..=1).speed(0.01))
                                    .changed()
                                {
                                    changed = true;
                                };
                            });
                        });
                    }
                })
            });

        EditorResponse::new(changed)
    }
}
