use crate::workspace::graph::viewer::default_view::DefaultNodeView;
use crate::workspace::graph::viewer::NodeView;
use crate::workspace::graph::GraphViewer;
use dbe_backend::graph::node::groups::tree_subgraph::TreeSubgraphFactory;
use dbe_backend::graph::node::{NodeFactory, SnarlNode};
use egui::{InnerResponse, Ui};
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, Snarl};
use ustr::Ustr;

#[derive(Debug)]
pub struct TreeSubgraphNodeViewer;

impl NodeView for TreeSubgraphNodeViewer {
    fn id(&self) -> Ustr {
        TreeSubgraphFactory.id()
    }

    fn show_input(
        &self,
        viewer: &mut GraphViewer,
        pin: &InPin,
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<InnerResponse<PinInfo>> {
        let res = DefaultNodeView.show_input(viewer, pin, ui, scale, snarl)?;

        res.response.context_menu(|_ui| todo!());

        Ok(res)
    }
}
