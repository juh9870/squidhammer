use crate::workspace::graph::viewer::NodeView;
use crate::workspace::graph::GraphViewer;
use dbe_backend::graph::node::generic::destructuring::DestructuringNodeFactory;
use dbe_backend::graph::node::{NodeFactory, SnarlNode};
use egui_snarl::ui::NodeLayout;
use ustr::Ustr;

#[derive(Debug)]
pub struct DestructuringNodeViewer;

impl NodeView for DestructuringNodeViewer {
    fn id(&self) -> Ustr {
        DestructuringNodeFactory.id()
    }

    fn node_layout(&self, _viewer: &mut GraphViewer, _node: &SnarlNode) -> NodeLayout {
        NodeLayout::Sandwich
    }
}
