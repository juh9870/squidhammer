use crate::widgets::report::diagnostics_column;
use crate::workspace::editors::utils::{
    inline_error, labeled_error, labeled_field, prop, unsupported, EditorSize,
};
use crate::workspace::editors::{
    cast_props, editor_for_item, DynProps, Editor, EditorData, EditorProps, EditorResponse,
};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::etype::econst::ETypeConst;
use dbe_backend::etype::eenum::pattern::EnumPattern;
use dbe_backend::etype::eenum::variant::{EEnumVariant, EEnumVariantId, EEnumVariantWithId};
use dbe_backend::etype::eenum::EEnumData;
use dbe_backend::etype::eitem::EItemInfo;
use dbe_backend::registry::ETypesRegistry;
use dbe_backend::value::EValue;
use egui::collapsing_header::CollapsingState;
use egui::{Align, Direction, Ui};
use miette::{bail, miette};
use tracing::error;

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
    fn props(&self, _reg: &ETypesRegistry, item: Option<&EItemInfo>) -> miette::Result<DynProps> {
        let kind = prop(
            item.map(|i| i.extra_properties()),
            "kind",
            EnumEditorType::Auto,
        )?;

        Ok(EnumEditorProps { ty: kind }.pack())
    }

    fn size(&self, _props: &DynProps) -> EditorSize {
        EditorSize::Block
    }

    fn edit(
        &self,
        ui: &mut Ui,
        reg: &ETypesRegistry,
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

        let Some(mut editor) =
            EnumEditorData::init(ui, reg, diagnostics, field_name, variant, value)
        else {
            return EditorResponse::unchanged();
        };
        editor.hide_const_body();

        match props.ty {
            EnumEditorType::Toggle | EnumEditorType::Auto if editor.is_auto_toggle() => {
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
                        labeled_field(ui, field_name, |ui| editor.toggle_editor(ui))
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
                            labeled_field(ui, field_name, |ui| editor.toggle_editor(ui));
                            editor.body(ui)
                        },
                    )
                    .inner
                }
            }
            _ => {
                if editor.body_size().is_block() {
                    CollapsingState::load_with_default_open(
                        ui.ctx(),
                        ui.id().with(field_name),
                        true,
                    )
                    .show_header(ui, |ui| {
                        labeled_field(ui, field_name, |ui| editor.picker(ui))
                    })
                    .body_unindented(|ui| editor.body(ui))
                    .2
                    .map(|r| r.inner)
                    .unwrap_or(EditorResponse::unchanged())
                } else {
                    labeled_field(ui, field_name, |ui| editor.picker(ui));
                    editor.body(ui)
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
    registry: &'a ETypesRegistry,
    diagnostics: DiagnosticContextRef<'a>,
    field_name: &'a str,
    variant: &'a mut EEnumVariantId,
    value: &'a mut EValue,

    editor: EditorData,

    skip_draw_body: bool,
    enum_data: &'a EEnumData,
    selected_variant: &'a EEnumVariant,
}

impl<'a> EnumEditorData<'a> {
    pub fn init(
        ui: &mut Ui,
        registry: &'a ETypesRegistry,
        diagnostics: DiagnosticContextRef<'a>,
        field_name: &'a str,
        variant: &'a mut EEnumVariantId,
        value: &'a mut EValue,
    ) -> Option<Self> {
        let Some((enum_data, selected_variant)) = variant.enum_variant(registry) else {
            labeled_error(ui, field_name, miette!("Failed to find enum variant"));
            return None;
        };

        let editor = editor_for_item(registry, &selected_variant.data);

        Some(Self {
            registry,
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

    fn is_auto_toggle(&self) -> bool {
        false
    }

    fn change_variant(&mut self, new_variant: EEnumVariantId) {
        *self.variant = new_variant;
        match new_variant.variant(self.registry) {
            None => {
                error!(id=?new_variant, "Failed to obtain enum variant for ID")
            }
            Some(variant) => {
                *self.value = variant.default_value(self.registry).into_owned();
            }
        }
    }

    fn picker(&mut self, ui: &mut Ui) {
        let mut selected = *self.variant;
        egui::ComboBox::from_id_salt(self.field_name)
            .selected_text(self.selected_variant.name.as_str())
            // .width(ui.available_width())
            .show_ui(ui, |ui| {
                for (variant, id) in self.enum_data.variants_with_ids() {
                    ui.selectable_value(&mut selected, *id, variant.name.as_str());
                }
            });
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
                    first.0.name.as_str()
                } else {
                    second.0.name.as_str()
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

        let checked = self.variant == first.1;

        let mut after_check = checked;
        checkbox(ui, &mut after_check, first, second);
        if after_check != checked {
            self.change_variant(if after_check { *first.1 } else { *second.1 })
        }
    }

    fn body(mut self, ui: &mut Ui) -> EditorResponse {
        if !self.skip_draw_body {
            let mut d = self
                .diagnostics
                .enter_variant(self.selected_variant.name.as_str());

            let res = self
                .editor
                .show(ui, self.registry, d.enter_inline(), "", self.value);

            diagnostics_column(ui, d.get_reports_shallow());

            res
        } else {
            EditorResponse::unchanged()
        }
    }
}
