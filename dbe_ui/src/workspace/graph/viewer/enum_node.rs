use crate::workspace::graph::viewer::NodeView;
use crate::workspace::graph::GraphViewer;
use dbe_backend::graph::node::enum_node::EnumNode;
use dbe_backend::graph::node::SnarlNode;
use egui::Ui;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use miette::bail;
use ustr::Ustr;

#[derive(Debug)]
pub struct EnumNodeViewer;

impl NodeView for EnumNodeViewer {
    fn id(&self) -> Ustr {
        "enum_node".into()
    }

    fn has_body(&self, _viewer: &mut GraphViewer, _node: &SnarlNode) -> miette::Result<bool> {
        Ok(true)
    }

    fn show_body(
        &self,
        viewer: &mut GraphViewer,
        node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<()> {
        let node = snarl[node_id]
            .downcast_mut::<EnumNode>()
            .expect("EnumNodeViewer should only be used with EnumNode");
        let mut variant_id = node.variant();

        let Some((data, variant)) = variant_id.enum_variant(viewer.ctx.registry) else {
            bail!("Enum variant {:?} not found", variant_id);
        };

        egui::ComboBox::new("variant", "Variant")
            .selected_text(variant.name())
            .show_ui(ui, |ui| {
                for (variant, id) in data.variants_with_ids() {
                    ui.selectable_value(&mut variant_id, *id, variant.name());
                }
            });

        node.set_variant(&mut viewer.commands, node_id, variant_id)?;
        viewer.commands.execute(&mut viewer.ctx, snarl)?;
        Ok(())
    }
}
