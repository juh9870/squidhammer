use crate::main_toolbar::docs::docs_hover;
use crate::ui_props::{PROP_FIELD_KIND, PROP_OBJECT_KIND};
use crate::widgets::report::diagnostics_column;
use crate::workspace::editors::utils::{
    inline_error, labeled_error, labeled_field, prop, unsupported, EditorSize,
};
use crate::workspace::editors::{
    cast_props, editor_for_item, DynProps, Editor, EditorContext, EditorData, EditorProps,
    EditorResponse,
};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::etype::econst::ETypeConst;
use dbe_backend::etype::eenum::pattern::EnumPattern;
use dbe_backend::etype::eenum::variant::{EEnumVariant, EEnumVariantId, EEnumVariantWithId};
use dbe_backend::etype::eenum::EEnumData;
use dbe_backend::etype::eitem::EItemInfo;
use dbe_backend::etype::property::ObjectPropertyId;
use dbe_backend::project::docs::DocsRef;
use dbe_backend::registry::ETypesRegistry;
use dbe_backend::value::EValue;
use egui::collapsing_header::CollapsingState;
use egui::{Align, Direction, Ui};
use miette::{bail, miette};
use std::ops::Deref;
use tracing::error;
use utils::map::HashMap;
use utils::whatever_ref::{WhateverRef, WhateverRefMap};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum EnumEditorType {
    Auto,
    Full,
    Toggle,
}

impl TryFrom<ETypeConst> for EnumEditorType {
    type Error = miette::Error;

    fn try_from(value: ETypeConst) -> Result<Self, Self::Error> {
        if let ETypeConst::String(str) = value {
            match str.as_str() {
                "auto" => return Ok(EnumEditorType::Auto),
                "full" => return Ok(EnumEditorType::Full),
                "toggle" => return Ok(EnumEditorType::Toggle),
                _ => {}
            }
        }
        bail!(
            "Expected one of `auto`, `full`, `toggle`, but got {:?}",
            value
        )
    }
}

#[derive(Debug)]
pub struct EnumEditor;

impl Editor for EnumEditor {
    fn props(
        &self,
        _reg: &ETypesRegistry,
        item: Option<&EItemInfo>,
        object_props: DynProps,
    ) -> miette::Result<DynProps> {
        let kind = prop(
            item.map(|i| i.extra_properties()),
            PROP_FIELD_KIND.deref(),
            EnumEditorType::Auto,
        )?;

        let kind = if kind == EnumEditorType::Auto {
            let obj_props = cast_props::<EnumEditorProps>(&object_props);
            obj_props.ty
        } else {
            kind
        };

        Ok(EnumEditorProps { ty: kind }.pack())
    }

    fn object_props(
        &self,
        _reg: &ETypesRegistry,
        props: &HashMap<ObjectPropertyId, ETypeConst>,
    ) -> miette::Result<DynProps> {
        let kind = prop(props, PROP_OBJECT_KIND.deref(), EnumEditorType::Auto)?;

        Ok(EnumEditorProps { ty: kind }.pack())
    }

    fn size(&self, _props: &DynProps) -> EditorSize {
        EditorSize::Block
    }

    fn edit(
        &self,
        ui: &mut Ui,
        mut ctx: EditorContext,
        diagnostics: DiagnosticContextRef,
        field_name: &str,
        value: &mut EValue,
        props: &DynProps,
    ) -> EditorResponse {
        let EValue::Enum {
            variant,
            data: value,
        } = value
        else {
            unsupported!(ui, field_name, value, self);
        };

        let props = cast_props::<EnumEditorProps>(props);
        let docs_ctx = ctx.replace_docs_ref(DocsRef::None);

        let Some(mut editor) =
            EnumEditorData::init(ui, ctx, diagnostics, field_name, variant, value)
        else {
            return EditorResponse::unchanged();
        };
        editor.hide_const_body();

        match props.ty {
            EnumEditorType::Toggle => {
                if !editor.can_be_toggle() {
                    labeled_error(
                        ui,
                        field_name,
                        miette!("toggle enum must have exactly 2 variants"),
                    );
                }
                if editor.body_size().is_block() {
                    CollapsingState::load_with_default_open(
                        ui.ctx(),
                        ui.id().with(field_name),
                        true,
                    )
                    .show_header(ui, |ui| {
                        labeled_field(ui, field_name, docs_ctx, |ui| editor.toggle_editor(ui))
                    })
                    .body_unindented(|ui| editor.body(ui))
                    .2
                    .map(|r| r.inner)
                    .unwrap_or(EditorResponse::unchanged())
                } else {
                    let dir = if editor.body_size() <= EditorSize::Inline {
                        Direction::LeftToRight
                    } else {
                        Direction::TopDown
                    };

                    ui.with_layout(
                        egui::Layout::from_main_dir_and_cross_align(dir, Align::Min),
                        |ui| {
                            labeled_field(ui, field_name, docs_ctx, |ui| editor.toggle_editor(ui));
                            editor.body(ui)
                        },
                    )
                    .inner
                }
            }
            _ => {
                if editor.body_size().is_block() {
                    ui.vertical(|ui| {
                        CollapsingState::load_with_default_open(
                            ui.ctx(),
                            ui.id().with(field_name),
                            true,
                        )
                        .show_header(ui, |ui| {
                            labeled_field(ui, field_name, docs_ctx, |ui| editor.picker(ui))
                        })
                        .body_unindented(|ui| editor.body(ui))
                        .2
                        .map(|r| r.inner)
                        .unwrap_or(EditorResponse::unchanged())
                    })
                    .inner
                } else {
                    ui.horizontal(|ui| {
                        labeled_field(ui, field_name, docs_ctx, |ui| editor.picker(ui));
                        editor.body(ui)
                    })
                    .inner
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnumEditorProps {
    ty: EnumEditorType,
}

impl EditorProps for EnumEditorProps {}

struct EnumEditorData<'a> {
    ctx: EditorContext<'a>,
    diagnostics: DiagnosticContextRef<'a>,
    field_name: &'a str,
    variant: &'a mut EEnumVariantId,
    value: &'a mut EValue,

    editor: EditorData,

    skip_draw_body: bool,
    enum_data: WhateverRef<'a, EEnumData>,
    selected_variant: WhateverRefMap<'a, EEnumData, EEnumVariant>,
}

impl<'a> EnumEditorData<'a> {
    pub fn init(
        ui: &mut Ui,
        ctx: EditorContext<'a>,
        diagnostics: DiagnosticContextRef<'a>,
        field_name: &'a str,
        variant: &'a mut EEnumVariantId,
        value: &'a mut EValue,
    ) -> Option<Self> {
        let Some((enum_data, selected_variant)) = variant.enum_variant(ctx.registry) else {
            labeled_error(ui, field_name, miette!("Failed to find enum variant"));
            return None;
        };

        let editor = editor_for_item(ctx.registry, &selected_variant.data);

        Some(Self {
            ctx,
            diagnostics,
            field_name,
            variant,
            value,
            editor,
            skip_draw_body: false,
            enum_data,
            selected_variant,
        })
    }

    fn hide_const_body(&mut self) {
        if matches!(self.selected_variant.pat, EnumPattern::Const(_)) {
            self.skip_draw_body = true;
        }
    }

    fn body_size(&self) -> EditorSize {
        if self.skip_draw_body {
            EditorSize::None
        } else {
            self.editor.size()
        }
    }

    fn can_be_toggle(&self) -> bool {
        self.enum_data.variants().len() == 2
    }

    fn change_variant(&mut self, new_variant: EEnumVariantId) {
        *self.variant = new_variant;
        match new_variant.variant(self.ctx.registry) {
            None => {
                error!(id=?new_variant, "Failed to obtain enum variant for ID")
            }
            Some(variant) => {
                *self.value = variant.default_value(self.ctx.registry).into_owned();
            }
        }
    }

    fn picker(&mut self, ui: &mut Ui) {
        let mut selected = *self.variant;
        let res = egui::ComboBox::from_id_salt(self.field_name)
            .selected_text(self.selected_variant.name.as_str())
            // .width(ui.available_width())
            .show_ui(ui, |ui| {
                for (variant, id) in self.enum_data.variants_with_ids() {
                    ui.selectable_value(&mut selected, *id, variant.name.as_str());
                }
            });

        docs_hover(
            ui,
            res.response,
            self.field_name,
            self.ctx.docs,
            self.ctx.registry,
            DocsRef::EnumVariant(selected.enum_id(), selected.variant_name()),
        );

        if &selected != self.variant {
            self.change_variant(selected)
        }
    }

    fn toggle_data(&self) -> miette::Result<(EEnumVariantWithId, EEnumVariantWithId)> {
        let mut iter = self.enum_data.variants_with_ids();
        let first = iter.next().ok_or_else(|| {
            miette!("Toggle enum editor requires exactly two enum variants, got zero")
        })?;
        let second = iter.next().ok_or_else(|| {
            miette!("Toggle enum editor requires exactly two enum variants, got one")
        })?;
        let count = iter.count();
        if count > 0 {
            bail!(
                "Toggle enum editor requires exactly two enum variants, got {}",
                count + 2
            )
        }
        Ok((first, second))
    }

    fn toggle_editor(&mut self, ui: &mut Ui) {
        self.toggle_editor_custom(ui, |ui, checked, first, second| {
            ui.toggle_value(
                checked,
                if *checked {
                    second.0.name.as_str()
                } else {
                    first.0.name.as_str()
                },
            );
        })
    }

    fn toggle_editor_custom(
        &mut self,
        ui: &mut Ui,
        checkbox: impl FnOnce(&mut Ui, &mut bool, EEnumVariantWithId, EEnumVariantWithId),
    ) {
        let (first, second) = match self.toggle_data() {
            Ok(data) => data,
            Err(err) => {
                inline_error(ui, err);
                return;
            }
        };

        let checked = self.variant == second.1;

        let mut after_check = checked;
        checkbox(ui, &mut after_check, first, second);
        if after_check != checked {
            self.change_variant(if after_check { *second.1 } else { *first.1 })
        }
    }

    fn body(mut self, ui: &mut Ui) -> EditorResponse {
        if !self.skip_draw_body {
            let mut d = self
                .diagnostics
                .enter_variant(self.selected_variant.name.as_str());

            let res = self
                .editor
                .show(ui, self.ctx, d.enter_inline(), "", self.value);

            diagnostics_column(ui, d.get_reports_shallow());

            res
        } else {
            EditorResponse::unchanged()
        }
    }
}
