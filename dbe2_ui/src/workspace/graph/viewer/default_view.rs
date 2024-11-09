use crate::workspace::editors::editor_for_value;
use crate::workspace::graph::viewer::NodeView;
use crate::workspace::graph::{pin_info, GraphViewer};
use dbe2::graph::node::SnarlNode;
use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use std::fmt::Debug;
use ustr::Ustr;

#[derive(Debug)]
pub struct DefaultNodeView;

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
            let reg = viewer.ctx.registry;
            let value = viewer.ctx.get_inline_input_mut(snarl, pin.id)?;
            let editor = editor_for_value(reg, value);
            let res = editor.show(
                ui,
                reg,
                viewer.diagnostics.enter_field(input_data.name.as_str()),
                &input_data.name,
                value,
            );

            if res.changed {
                viewer.ctx.mark_dirty(snarl, pin.id.node);
            }
        } else {
            let value = viewer.ctx.read_input(snarl, pin.id)?;
            ui.horizontal(|ui| {
                ui.label(&*input_data.name);
                ui.label(value.to_string());
            });
        }

        Ok(pin_info(input_data.ty, registry))
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
            ui.label(value.to_string());
        });

        Ok(pin_info(output_data.ty, registry))
    }
}
