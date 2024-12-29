use crate::main_toolbar::docs::{docs_label, DocsRef};
use crate::ui_props::PROP_OBJECT_GRAPH_INLINE;
use crate::workspace::editors::{editor_for_item, EditorContext};
use crate::workspace::graph::viewer::NodeView;
use crate::workspace::graph::{any_pin, pin_info, GraphViewer};
use dbe_backend::etype::eobject::EObject;
use dbe_backend::etype::EDataType;
use dbe_backend::graph::node::SnarlNode;
use dbe_backend::registry::ETypesRegistry;
use dbe_backend::value::EValue;
use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use std::fmt::Debug;
use ustr::Ustr;

#[derive(Debug)]
pub struct DefaultNodeView;

pub fn format_value(value: &EValue) -> String {
    match value {
        EValue::Null => "null".to_string(),
        EValue::Boolean { value } => value.to_string(),
        EValue::Number { value } => format!("{:.5}", value)
            .trim_end_matches(['0', '.'])
            .to_string(),
        EValue::String { value } => {
            if value.len() > 8 {
                format!("{:?}...", &value[..8])
            } else {
                format!("{:?}", value)
            }
        }
        EValue::Struct { .. } => "".to_string(),
        EValue::Enum { .. } => "".to_string(),
        EValue::List { .. } => "".to_string(),
        EValue::Map { .. } => "".to_string(),
    }
}

pub fn has_inline_editor(registry: &ETypesRegistry, ty: EDataType, editable: bool) -> bool {
    match ty {
        EDataType::Boolean => editable,
        EDataType::Number => editable,
        EDataType::String => editable,
        EDataType::Object { ident } => registry
            .get_object(&ident)
            .and_then(|obj| PROP_OBJECT_GRAPH_INLINE.try_get(obj.extra_properties()))
            .unwrap_or(editable),
        EDataType::Const { .. } => false,
        EDataType::List { id } => registry
            .get_list(&id)
            .map(|list| has_inline_editor(registry, list.value_type, editable))
            .unwrap_or(editable),
        EDataType::Map { id } => registry
            .get_map(&id)
            .map(|map| has_inline_editor(registry, map.value_type, editable))
            .unwrap_or(editable),
    }
}

impl NodeView for DefaultNodeView {
    fn id(&self) -> Ustr {
        "default".into()
    }

    fn show_header(
        &self,
        viewer: &GraphViewer,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<()> {
        ui.label(node.0.to_string());
        ui.label(snarl[node].title(viewer.ctx.as_node_context()));
        Ok(())
    }

    fn show_input(
        &self,
        viewer: &mut GraphViewer,
        pin: &InPin,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<PinInfo> {
        let registry = viewer.ctx.registry;
        let docs = viewer.ctx.docs.expect("Docs should be set at this point");
        let node = &snarl[pin.id.node];
        let node_ident = node.id();
        let input_data = node.try_input(viewer.ctx.as_node_context(), pin.id.input)?;
        let Some(info) = input_data.ty.item_info() else {
            return Ok(any_pin());
        };
        let ctx = EditorContext::new(
            registry,
            docs,
            DocsRef::NodeInput(node_ident, input_data.name.as_str()),
        );
        if pin.remotes.is_empty() {
            let mut full_ctx = viewer.ctx.as_full(snarl);
            if let Some(value) = full_ctx.get_inline_input_mut(pin.id)? {
                if has_inline_editor(registry, input_data.ty.ty(), true) {
                    let editor = editor_for_item(registry, info);
                    let res = ui.vertical(|ui| {
                        editor.show(
                            ui,
                            ctx,
                            viewer.diagnostics.enter_field(input_data.name.as_str()),
                            &input_data.name,
                            value,
                        )
                    });

                    if res.inner.changed {
                        full_ctx.mark_dirty(pin.id.node);
                    }
                } else {
                    ui.horizontal(|ui| {
                        docs_label(ui, &input_data.name, docs, registry, DocsRef::None);
                        ui.label(format_value(value));
                    });
                }
            }
        } else {
            ui.horizontal(|ui| {
                docs_label(ui, &input_data.name, docs, registry, DocsRef::None);
            });
        }

        Ok(pin_info(&input_data.ty, registry))
    }

    fn show_output(
        &self,
        viewer: &mut GraphViewer,
        pin: &OutPin,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<PinInfo> {
        let registry = viewer.ctx.registry;
        let node = &snarl[pin.id.node];
        let output_data = node.try_output(viewer.ctx.as_node_context(), pin.id.output)?;
        let docs = viewer.ctx.docs.expect("Docs should be set at this point");
        ui.horizontal(|ui| {
            docs_label(
                ui,
                &output_data.name,
                docs,
                registry,
                DocsRef::NodeOutput(node.id(), output_data.name.as_str()),
            );
        });

        Ok(pin_info(&output_data.ty, registry))
    }

    fn has_body(&self, _viewer: &mut GraphViewer, _node: &SnarlNode) -> miette::Result<bool> {
        Ok(false)
    }

    fn show_body(
        &self,
        _viewer: &mut GraphViewer,
        _node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        _ui: &mut Ui,
        _scale: f32,
        _snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<()> {
        Ok(())
    }
}
