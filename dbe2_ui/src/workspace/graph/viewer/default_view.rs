use crate::workspace::editors::editor_for_item;
use crate::workspace::graph::viewer::NodeView;
use crate::workspace::graph::{pin_info, GraphViewer};
use dbe2::etype::EDataType;
use dbe2::graph::node::SnarlNode;
use dbe2::registry::ETypesRegistry;
use dbe2::value::EValue;
use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use std::fmt::Debug;
use ustr::Ustr;

#[derive(Debug)]
pub struct DefaultNodeView;

fn format_value(value: &EValue) -> String {
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

fn has_inline_editor(registry: &ETypesRegistry, ty: EDataType) -> bool {
    match ty {
        EDataType::Boolean => true,
        EDataType::Number => true,
        EDataType::String => true,
        EDataType::Object { ident } => registry
            .get_object(&ident)
            .map(|obj| {
                obj.extra_properties()
                    .get("graph_inline")
                    .is_some_and(|v| v.as_bool().is_some_and(|v| v))
            })
            .unwrap_or(false),
        EDataType::Const { .. } => false,
        EDataType::List { id } => registry
            .get_list(&id)
            .map(|list| has_inline_editor(registry, list.value_type))
            .unwrap_or(false),
        EDataType::Map { id } => registry
            .get_map(&id)
            .map(|map| has_inline_editor(registry, map.value_type))
            .unwrap_or(false),
    }
}

impl NodeView for DefaultNodeView {
    fn id(&self) -> Ustr {
        "default".into()
    }

    fn show_header(
        &self,
        _viewer: &GraphViewer,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<()> {
        ui.label(node.0.to_string());
        ui.label(snarl[node].title());
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
        let node = &snarl[pin.id.node];
        let input_data = node.try_input(viewer.ctx.registry, pin.id.input)?;
        if pin.remotes.is_empty() {
            let value = viewer.ctx.get_inline_input_mut(snarl, pin.id)?;
            if has_inline_editor(registry, input_data.ty.ty()) {
                let editor = editor_for_item(registry, &input_data.ty);
                let res = ui.vertical(|ui| {
                    editor.show(
                        ui,
                        registry,
                        viewer.diagnostics.enter_field(input_data.name.as_str()),
                        &input_data.name,
                        value,
                    )
                });

                if res.inner.changed {
                    viewer.ctx.mark_dirty(snarl, pin.id.node);
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(&*input_data.name);
                    ui.label(format_value(value));
                });
            }
        } else {
            let mut value = viewer.ctx.read_input(snarl, pin.id)?;
            if has_inline_editor(registry, input_data.ty.ty()) {
                let editor = editor_for_item(registry, &input_data.ty);
                ui.add_enabled_ui(true, |ui| {
                    ui.vertical(|ui| {
                        editor.show(
                            ui,
                            registry,
                            viewer.diagnostics.enter_field(input_data.name.as_str()),
                            &input_data.name,
                            &mut value,
                        )
                    });
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label(&*input_data.name);
                    ui.label(format_value(&value));
                });
            }
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
        let output_data = node.try_output(viewer.ctx.registry, pin.id.output)?;
        let value = viewer.ctx.read_output(snarl, pin.id)?;
        ui.horizontal(|ui| {
            ui.label(&*output_data.name);
            ui.label(format_value(&value));
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
