use crate::workspace::graph::viewer::default_view::DefaultNodeView;
use crate::workspace::graph::viewer::enum_node::EnumNodeViewer;
use crate::workspace::graph::viewer::reroute::RerouteViewer;
use crate::workspace::graph::GraphViewer;
use atomic_refcell::AtomicRefCell;
use dbe2::graph::node::SnarlNode;
use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::{Arc, LazyLock};
use ustr::{Ustr, UstrMap};

pub mod default_view;
pub mod enum_node;
pub mod reroute;

static NODE_VIEWERS: LazyLock<AtomicRefCell<UstrMap<Arc<dyn NodeView>>>> =
    LazyLock::new(|| AtomicRefCell::new(default_viewers().collect()));
static DEFAULT_VIEWER: LazyLock<Arc<dyn NodeView>> = LazyLock::new(|| Arc::new(DefaultNodeView));

fn default_viewers() -> impl Iterator<Item = (Ustr, Arc<dyn NodeView>)> {
    let v: Vec<Arc<dyn NodeView>> = vec![Arc::new(RerouteViewer), Arc::new(EnumNodeViewer)];
    v.into_iter().map(|item| (Ustr::from(&item.id()), item))
}

pub fn get_viewer(id: &Ustr) -> Arc<dyn NodeView> {
    NODE_VIEWERS
        .borrow()
        .get(id)
        .cloned()
        .unwrap_or_else(|| Arc::clone(DEFAULT_VIEWER.deref()))
}

pub trait NodeView: Send + Sync + Debug + 'static {
    fn id(&self) -> Ustr;

    #[allow(clippy::too_many_arguments)]
    fn show_header(
        &self,
        viewer: &GraphViewer,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<()> {
        DefaultNodeView.show_header(viewer, node, inputs, outputs, ui, scale, snarl)
    }

    fn show_input(
        &self,
        viewer: &mut GraphViewer,
        pin: &InPin,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<PinInfo> {
        DefaultNodeView.show_input(viewer, pin, ui, _scale, snarl)
    }

    fn show_output(
        &self,
        viewer: &mut GraphViewer,
        pin: &OutPin,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<PinInfo> {
        DefaultNodeView.show_output(viewer, pin, ui, _scale, snarl)
    }

    fn has_body(&self, viewer: &mut GraphViewer, node: &SnarlNode) -> miette::Result<bool> {
        DefaultNodeView.has_body(viewer, node)
    }

    #[allow(clippy::too_many_arguments)]
    fn show_body(
        &self,
        viewer: &mut GraphViewer,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<()> {
        DefaultNodeView.show_body(viewer, node, inputs, outputs, ui, scale, snarl)
    }
}
