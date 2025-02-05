use crate::main_toolbar::docs::docs_label;
use crate::ui_props::PROP_FIELD_HIDE_FIELDS;
use crate::widgets::report::diagnostics_column;
use crate::workspace::editors::utils::{labeled_field, unsupported, EditorResultExt, EditorSize};
use crate::workspace::editors::{
    cast_props, editor_for_item, DynProps, Editor, EditorContext, EditorProps, EditorResponse,
};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::etype::eitem::EItemInfo;
use dbe_backend::etype::property::default_properties::PROP_FIELD_INLINE;
use dbe_backend::project::docs::DocsRef;
use dbe_backend::registry::ETypesRegistry;
use dbe_backend::value::EValue;
use egui::{Label, RichText, Ui};
use itertools::Itertools;
use miette::miette;

#[derive(Debug)]
pub struct StructEditor;

impl Editor for StructEditor {
    fn props(
        &self,
        _reg: &ETypesRegistry,
        item: Option<&EItemInfo>,
        _object_props: DynProps,
    ) -> miette::Result<DynProps> {
        let props = item.map(|i| i.extra_properties());
        let inline = props.and_then(|p| PROP_FIELD_INLINE.try_get(p));
        let hide_fields = props.and_then(|p| PROP_FIELD_HIDE_FIELDS.try_get(p));
        Ok(StructProps {
            inline: inline.unwrap_or(false),
            hide_fields: hide_fields
                .map(|f| f.as_str().split(',').collect_vec())
                .unwrap_or_default(),
        }
        .pack())
    }

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
        props: &DynProps,
    ) -> EditorResponse {
        let EValue::Struct { fields, ident } = value else {
            unsupported!(ui, field_name, value, self);
        };

        let props = cast_props::<StructProps>(props);
        let docs_ctx = ctx.replace_docs_ref(DocsRef::None);

        let mut changed = false;
        ctx.registry
            .get_struct(ident)
            .ok_or_else(|| miette!("unknown struct `{}`", ident))
            .then_draw(ui, |ui, data| {
                let items = data
                    .fields
                    .iter()
                    .filter(|f| !props.hide_fields.contains(&f.name.as_str()))
                    .map(|f| (f, editor_for_item(ctx.registry, &f.ty)))
                    .collect_vec();

                let hidden = data.fields.len() - items.len();

                let inline = hidden == 0
                    && items.len() <= 3
                    && items.iter().all(|f| f.1.size() <= EditorSize::Inline);

                let draw_fields = |ui: &mut Ui| {
                    if hidden > 0 {
                        ui.add_enabled(
                            false,
                            Label::new(RichText::new(format!("{} fields hidden", hidden)).small()),
                        );
                    }
                    for (field, editor) in items {
                        ui.push_id(field.name, |ui| {
                            fields
                                .get_mut(&field.name)
                                .ok_or_else(|| miette!("field `{}` is missing", field.name))
                                .then_draw(ui, |ui, value| {
                                    let mut d = diagnostics.enter_field(field.name.as_str());
                                    let ctx =
                                        ctx.copy_with_docs(DocsRef::TypeField(*ident, field.name));
                                    if editor
                                        .show(ui, ctx, d.enter_inline(), field.name.as_ref(), value)
                                        .changed
                                    {
                                        changed = true;
                                    };
                                    diagnostics_column(ui, d.get_reports_shallow())
                                });
                        });
                    }
                };

                if inline {
                    ui.horizontal(|ui| {
                        labeled_field(ui, field_name, docs_ctx, draw_fields);
                    });
                } else if props.inline {
                    draw_fields(ui);
                } else if field_name.is_empty() {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            draw_fields(ui);
                        })
                    });
                } else {
                    egui::collapsing_header::CollapsingState::load_with_default_open(
                        ui.ctx(),
                        ui.id().with(field_name),
                        true,
                    )
                    .show_header(ui, |ui| {
                        docs_label(
                            ui,
                            field_name,
                            docs_ctx.docs,
                            docs_ctx.registry,
                            docs_ctx.docs_ref,
                        );
                    })
                    .body(|ui| {
                        ui.vertical(|ui| {
                            draw_fields(ui);
                        })
                    });
                }
            });

        EditorResponse::new(changed)
    }
}

#[derive(Debug, Clone)]
struct StructProps {
    inline: bool,
    hide_fields: Vec<&'static str>,
}

impl EditorProps for StructProps {}
