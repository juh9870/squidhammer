use crate::workspace::graph::viewer::NodeView;
use crate::workspace::graph::{pin_info, GraphViewer};
use dbe_backend::graph::node::SnarlNode;
use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use ustr::Ustr;

#[derive(Debug)]
pub struct RerouteViewer;

impl NodeView for RerouteViewer {
    fn id(&self) -> Ustr {
        "reroute".into()
    }

    fn show_header(
        &self,
        _viewer: &GraphViewer,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        _snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<()> {
        ui.label(node.0.to_string());
        Ok(())
    }

    fn show_input(
        &self,
        viewer: &mut GraphViewer,
        pin: &InPin,
        _ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<PinInfo> {
        let registry = viewer.ctx.registry;
        let node = &snarl[pin.id.node];
        let input_data = node.try_input(viewer.ctx.registry, pin.id.input)?;
        Ok(pin_info(&input_data.ty, registry))
    }

    fn show_output(
        &self,
        viewer: &mut GraphViewer,
        pin: &OutPin,
        _ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<PinInfo> {
        let registry = viewer.ctx.registry;
        let node = &snarl[pin.id.node];
        let output_data = node.try_output(viewer.ctx.registry, pin.id.output)?;
        // let value = viewer.ctx.read_output(snarl, pin.id)?;
        // if value != EValue::Null {
        //     ui.label(value.to_string());
        // }

        Ok(pin_info(&output_data.ty, registry))
    }
}
