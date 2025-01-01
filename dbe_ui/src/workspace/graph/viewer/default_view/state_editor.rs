use crate::main_toolbar::docs::{docs_hover, docs_label};
use crate::workspace::editors::{editor_for_value, EditorContext, EditorResponse};
use crate::workspace::graph::GraphViewer;
use dbe_backend::etype::eenum::variant::EEnumVariantId;
use dbe_backend::graph::node::editable_state::{EditableState, EditableStateValue};
use dbe_backend::project::docs::DocsRef;
use egui::Ui;
use miette::bail;
use ustr::Ustr;

pub fn show_state_editor(
    ui: &mut Ui,
    viewer: &mut GraphViewer,
    node_id: Ustr,
    state: &mut EditableState,
) -> miette::Result<EditorResponse> {
    ui.vertical(|ui| {
        let mut res = EditorResponse::unchanged();
        for (ref field_name, ref mut value) in state.iter_mut() {
            match value {
                EditableStateValue::Value(value) => {
                    let editor = editor_for_value(viewer.ctx.registry, value);
                    let ctx = EditorContext {
                        registry: viewer.ctx.registry,
                        docs: viewer.ctx.docs,
                        docs_ref: DocsRef::NodeState(node_id, *field_name),
                    };
                    res |= editor.show(
                        ui,
                        ctx,
                        viewer.diagnostics.enter_inline(),
                        field_name,
                        value,
                    );
                }
                EditableStateValue::EnumVariant(variant) => {
                    res |= enum_variant_editor(ui, viewer, node_id, *field_name, variant)?;
                }
            }
        }
        Ok(res)
    })
    .inner
}

fn enum_variant_editor(
    ui: &mut Ui,
    viewer: &mut GraphViewer,
    node_id: Ustr,
    field_name: Ustr,
    variant_id: &mut EEnumVariantId,
) -> miette::Result<EditorResponse> {
    let Some((data, variant)) = variant_id.enum_variant(viewer.ctx.registry) else {
        bail!("Enum variant {:?} not found", variant_id);
    };

    let mut new_id = *variant_id;

    ui.horizontal(|ui| {
        docs_label(
            ui,
            field_name.as_str(),
            viewer.ctx.docs,
            viewer.ctx.registry,
            DocsRef::NodeState(node_id, field_name),
        );
        let combo_res = egui::ComboBox::from_id_salt(field_name)
            .selected_text(variant.name())
            .show_ui(ui, |ui| {
                for (variant, id) in data.variants_with_ids() {
                    ui.selectable_value(&mut new_id, *id, variant.name());
                }
            });

        docs_hover(
            ui,
            combo_res.response,
            field_name.as_str(),
            viewer.ctx.docs,
            viewer.ctx.registry,
            DocsRef::EnumVariant(variant_id.enum_id(), variant_id.variant_name()),
        );
    });

    if new_id != *variant_id {
        *variant_id = new_id;
        Ok(EditorResponse::changed())
    } else {
        Ok(EditorResponse::unchanged())
    }
}
