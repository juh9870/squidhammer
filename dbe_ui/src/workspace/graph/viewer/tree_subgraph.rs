use crate::workspace::graph::viewer::default_view::DefaultNodeView;
use crate::workspace::graph::viewer::NodeView;
use crate::workspace::graph::GraphViewer;
use dbe_backend::graph::node::groups::tree_subgraph::{TreeSubgraph, TreeSubgraphFactory};
use dbe_backend::graph::node::{NodeFactory, SnarlNode};
use egui::{vec2, InnerResponse, Rect, Stroke, Ui};
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, Snarl};
use inline_tweak::tweak;
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
        let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        let color = stroke.color;
        let rect_size = tweak!(4.0) * scale;
        let shrink = (rect_size - stroke.width) / 2.0;
        let margin = ui.style().spacing.item_spacing.y;

        let node = snarl[pin.id.node].downcast_mut::<TreeSubgraph>().unwrap();
        let state = node.tree_cache(viewer.ctx.as_node_context());
        let start_of = &state.start_of[pin.id.input].clone();
        let end_of = &state.end_of[pin.id.input].clone();
        let hierarchy = &state.hierarchy[pin.id.input].clone();
        let hierarchy_width = state.width;
        // let _guard = trace_span!("tree_node_show_input", input=pin.id.input, ?hierarchy_width, hierarchy_len=?hierarchy.len()).entered();

        let full_width = ui.available_width();

        let res = ui
            .vertical(|ui| -> miette::Result<_> {
                for (idx, _) in start_of.iter().rev().enumerate() {
                    let pos = ui.cursor().min;
                    let offset = hierarchy.len() - start_of.len() + idx;
                    // trace!(?offset, ?node, ?idx, "start of input");
                    let width = full_width - rect_size * offset as f32;
                    ui.painter().rect(
                        Rect::from_min_size(pos, vec2(width, rect_size)).shrink(shrink),
                        0.0,
                        color,
                        Stroke::NONE,
                    );
                    ui.painter().rect(
                        Rect::from_min_size(
                            pos + vec2(width - rect_size, rect_size / 2.0),
                            vec2(
                                rect_size,
                                rect_size * (start_of.len() - idx) as f32 + margin,
                            ),
                        )
                        .shrink2(vec2(shrink, 0.0)),
                        0.0,
                        color,
                        Stroke::NONE,
                    );
                    ui.add_space(rect_size);
                }
                if !start_of.is_empty() {
                    ui.add_space(margin);
                }

                let strokes_pos = ui.cursor().min;

                let res = DefaultNodeView.show_input(viewer, pin, ui, scale, snarl)?;

                for (idx, id) in hierarchy.iter().enumerate() {
                    let offset = hierarchy.len() - idx;
                    // trace!(?offset, node=?id, ?idx, "right side stroke");
                    let mut pos =
                        strokes_pos + vec2(full_width - rect_size * offset as f32, rect_size / 2.0);
                    let mut size = vec2(rect_size, res.response.rect.height());
                    if !start_of.is_empty() && idx >= start_of.len() {
                        let expand = margin + rect_size * start_of.len() as f32;
                        pos.y -= expand;
                        size.y += expand;
                    }
                    if !end_of.is_empty() && idx >= end_of.len() {
                        let expand = margin * 2.0 + rect_size * end_of.len() as f32;
                        size.y += expand;
                    } else if end_of.is_empty() {
                        size.y += margin;
                    }
                    ui.painter().rect(
                        Rect::from_min_size(pos, size).shrink2(vec2(shrink, 0.0)),
                        0.0,
                        color,
                        Stroke::NONE,
                    );
                }

                for (idx, _) in end_of.iter().rev().enumerate().rev() {
                    let pos = ui.cursor().min;
                    let offset = hierarchy.len() - end_of.len() + idx;
                    // trace!(?offset, ?node, ?idx, "start of input");
                    let width = full_width - rect_size * offset as f32;
                    ui.painter().rect(
                        Rect::from_min_size(pos, vec2(width, rect_size)).shrink(shrink),
                        0.0,
                        color,
                        Stroke::NONE,
                    );
                    let height = rect_size * (end_of.len() - idx) as f32;
                    ui.painter().rect(
                        Rect::from_min_size(
                            pos + vec2(width - rect_size, -height + rect_size / 2.0),
                            vec2(rect_size, height),
                        )
                        .shrink2(vec2(shrink, 0.0)),
                        0.0,
                        color,
                        Stroke::NONE,
                    );
                    ui.add_space(rect_size);
                }

                Ok(res)
            })
            .inner?;

        ui.allocate_ui(
            vec2(
                rect_size * hierarchy.len() as f32,
                res.response.rect.height(),
            ),
            |_| {},
        );
        Ok(res)
    }
}
