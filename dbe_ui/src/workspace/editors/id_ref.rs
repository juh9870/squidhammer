use crate::m_try;
use crate::ui_props::{PROP_FIELD_SHOW_FIELD_PATH, PROP_FIELD_SHOW_FILE_PATH};
use crate::widgets::dropdown::DropDownBox;
use crate::workspace::editors::utils::{labeled_field, unsupported, EditorResultExt, EditorSize};
use crate::workspace::editors::{
    cast_props, DynProps, Editor, EditorContext, EditorProps, EditorResponse,
};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::etype::eitem::EItemInfo;
use dbe_backend::etype::EDataType;
use dbe_backend::registry::ETypesRegistry;
use dbe_backend::validation::ids::numeric::NumericIDRegistry;
use dbe_backend::value::{ENumber, EValue};
use egui::{Frame, Ui, Widget};
use egui_hooks::UseHookExt;
use miette::bail;
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct IdRefEditor;

impl Editor for IdRefEditor {
    fn props(&self, _reg: &ETypesRegistry, item: Option<&EItemInfo>) -> miette::Result<DynProps> {
        let props = item.map(|i| i.extra_properties());
        let show_file_path = props
            .and_then(|p| PROP_FIELD_SHOW_FILE_PATH.try_get(p))
            .unwrap_or(false);
        let show_field_path = props
            .and_then(|p| PROP_FIELD_SHOW_FIELD_PATH.try_get(p))
            .unwrap_or(false);

        Ok(IdRefProps {
            show_file_path,
            show_field_path,
        }
        .pack())
    }

    fn size(&self, _props: &DynProps) -> EditorSize {
        EditorSize::Inline
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
        let EValue::Struct { fields, ident: ty } = value else {
            unsupported!(ui, field_name, value, self);
        };

        let props = cast_props::<IdRefProps>(props);
        let data_ty = EDataType::Object { ident: *ty };
        let mut changed = false;

        m_try(|| {
            if fields.len() != 1 {
                bail!("expected exactly one field, found {}", fields.len());
            };

            let Some(value) = fields.get_mut(&"id".into()) else {
                bail!("expected field `id`");
            };

            let EValue::Number { value } = value else {
                bail!("expected number, found {:?}", value);
            };

            let id_name = NumericIDRegistry::of(ctx.registry)
                .location_for_id(data_ty, *value)?
                .map(|name| format_name(&name, props.show_file_path, props.show_field_path));

            Ok((value, id_name))
        })
        .then_draw(ui, |ui, (value, name)| {
            let registry = ctx.registry;
            labeled_field(ui, field_name, ctx, |ui| {
                let mut edited_text = ui.use_state(|| None::<String>, ()).into_var();
                if let Some(edited) = edited_text.as_mut() {
                    if let Some(done) = NumericIDRegistry::of(registry)
                        .with_available_ids(data_ty, |iter| {
                            let res = DropDownBox::from_iter(
                                iter.map(|(id, locations)| {
                                    DropDownItem::new(
                                        id,
                                        name_of(locations, true, props.show_field_path),
                                    )
                                }),
                                field_name,
                                edited,
                                |ui, id, name| {
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                    let res = ui.selectable_label(id.num == value, name);
                                    if res.clicked() {
                                        *value = *id.num;
                                        changed = true;
                                    }
                                    res
                                },
                            )
                            .ui(ui);

                            // res.request_focus();

                            Ok(res.clicked_elsewhere())
                        })
                        .or_draw_error(ui)
                    {
                        if done {
                            *edited_text = None;
                        }
                    };
                } else {
                    Frame {
                        fill: ui.style().visuals.code_bg_color,
                        ..Default::default()
                    }
                    .show(ui, |ui| {
                        if ui
                            .selectable_label(false, name.unwrap_or_else(|| value.to_string()))
                            .clicked()
                        {
                            *edited_text = Some(value.to_string());
                        };
                    });
                }
            });
        });

        EditorResponse::new(changed)
    }
}

struct DropDownItem<'a> {
    num: &'a ENumber,
    ref_name: String,
}

impl<'a> DropDownItem<'a> {
    pub fn new(num: &'a ENumber, ref_name: Option<String>) -> Self {
        Self {
            num,
            ref_name: if let Some(name) = ref_name {
                format!("{} ({})", num, name)
            } else {
                num.to_string()
            },
        }
    }
}

impl AsRef<str> for DropDownItem<'_> {
    fn as_ref(&self) -> &str {
        &self.ref_name
    }
}

#[derive(Debug, Clone)]
struct IdRefProps {
    show_file_path: bool,
    show_field_path: bool,
}

impl EditorProps for IdRefProps {}

fn name_of(
    locations: Option<&BTreeSet<String>>,
    show_file_path: bool,
    show_field_path: bool,
) -> Option<String> {
    locations
        .and_then(|l| l.iter().next())
        .map(|loc| format_name(loc, show_file_path, show_field_path))
}

fn format_name(location: &str, show_file_path: bool, show_field_path: bool) -> String {
    if show_field_path && show_file_path {
        return location.to_string();
    }

    let mut parts = location.split('@');
    let path = parts.next().expect("Should have at least one segment");
    let field_path = parts.next();

    let trunc = if show_file_path {
        path
    } else {
        path.split('/')
            .last()
            .expect("Should have at least one segment")
    };

    if show_field_path {
        format!("{}@{}", trunc, field_path.unwrap_or(""))
    } else {
        trunc.to_string()
    }
}
