use crate::workspace::editors::utils::{
    labeled_collapsing_header, unsupported, EditorResultExt, EditorSize,
};
use crate::workspace::editors::{editor_for_type, DynProps, Editor, EditorContext, EditorResponse};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::etype::econst::ETypeConst;
use dbe_backend::etype::eenum::variant::EEnumVariantId;
use dbe_backend::etype::EDataType;
use dbe_backend::project::docs::DocsRef;
use dbe_backend::value::EValue;
use egui::Ui;
use miette::{bail, miette};

#[derive(Debug)]
pub struct EnumFlagsEditor;

impl Editor for EnumFlagsEditor {
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

        let bool_edit = editor_for_type(ctx.registry, &EDataType::Boolean);

        ctx.registry
            .get_list(id)
            .ok_or_else(|| miette!("!!INTERNAL ERROR!! unknown list `{}`", id))
            .and_then(|list| {
                let EDataType::Object { ident } = list.value_type else {
                    bail!(
                        "!!INTERNAL ERROR!! expected object type, got {:?}",
                        list.value_type
                    )
                };

                let enum_data = ctx
                    .registry
                    .get_enum(&ident)
                    .ok_or_else(|| miette!("!!INTERNAL ERROR!! unknown enum `{}`", ident))?;

                let mut flags = Vec::with_capacity(enum_data.variants().len());
                for (variant, id) in enum_data.variants_with_ids() {
                    let EDataType::Const { value } = variant.data.ty() else {
                        bail!(
                            "enum_flags editor only supports lists of enums with const variants, found variant `{}` with type `{}`",
                            variant.name(),
                            variant.data.ty().name()
                        )
                    };
                    flags.push(Flag {
                        id: *id,
                        ty: value,
                    });
                }

                Ok(flags)
            })
            .then_draw(ui, |ui, flags| {
                labeled_collapsing_header(ui, field_name, docs_ctx, true, false, |ui| {
                    for flag in flags {
                        let value = flag.as_value();
                        let flag_was_set = values.contains(&value);
                        let mut bool_val = EValue::Boolean { value: flag_was_set };

                        bool_edit.show(ui,
                                       ctx.copy_with_docs(DocsRef::EnumVariant(
                                           flag.id.enum_id(),
                                           flag.id.variant_name(),
                                       )),
                                       diagnostics.enter_inline(),
                                       flag.id.variant_name().as_str(),
                                       &mut bool_val,
                        );

                        let new_flag_set = *bool_val.try_as_boolean().expect("Boolean editor should not change underlying data type");

                        if new_flag_set != flag_was_set {
                            if new_flag_set {
                                values.push(value);
                            } else {
                                values.retain(|v| v != &value);
                            }
                            changed = true;
                        }
                    }
                })
            });

        EditorResponse::new(changed)
    }
}

#[derive(Debug, Copy, Clone)]
struct Flag {
    id: EEnumVariantId,
    ty: ETypeConst,
}

impl Flag {
    fn as_value(&self) -> EValue {
        EValue::Enum {
            variant: self.id,
            data: Box::new(self.ty.default_value()),
        }
    }
}
