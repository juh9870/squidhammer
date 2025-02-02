use crate::widgets::report::diagnostics_column;
use crate::workspace::editors::utils::{
    labeled_collapsing_header, unsupported, EditorResultExt, EditorSize,
};
use crate::workspace::editors::{editor_for_type, DynProps, Editor, EditorContext, EditorResponse};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::project::docs::DocsRef;
use dbe_backend::value::EValue;
use egui::Ui;
use miette::miette;

#[derive(Debug)]
pub struct ListEditor;

impl Editor for ListEditor {
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
        let EValue::List { values, id } = value else {
            unsupported!(ui, field_name, value, self);
        };

        let mut changed = false;
        let docs_ctx = ctx.replace_docs_ref(DocsRef::None);

        ctx.registry
            .get_list(id)
            .ok_or_else(|| miette!("!!INTERNAL ERROR!! unknown list `{}`", id))
            .then_draw(ui, |ui, list_data| {
                labeled_collapsing_header(
                    ui,
                    field_name,
                    docs_ctx,
                    values.len() < 20,
                    true,
                    |ui| {
                        let ty = list_data.value_type;
                        let editor = editor_for_type(ctx.registry, &ty);
                        list_edit::list_editor::<EValue, _>(ui.id().with(field_name).with("list"))
                            .new_item(|_| ty.default_value(ctx.registry).into_owned())
                            .show(ui, values, |ui, i, val| {
                                let mut d = diagnostics.enter_index(i.index);
                                if editor
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

                                diagnostics_column(ui, d.get_reports_shallow());
                            });
                    },
                );
            });

        EditorResponse::new(changed)
    }
}
