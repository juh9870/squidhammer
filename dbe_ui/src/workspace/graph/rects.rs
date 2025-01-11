use ahash::{AHashMap, AHasher};
use dbe_backend::graph::region::region_graph::RegionGraphData;
use egui::emath::OrderedFloat;
use egui::{Pos2, Rect};
use egui_snarl::ui::Viewport;
use egui_snarl::NodeId;
use inline_tweak::tweak;
use std::hash::{Hash, Hasher};
use utils::math::convex_hull_2d::{Convex, ConvexHull2D};
use uuid::Uuid;

/// Stores node rects in screen space
#[derive(Debug, Clone, Default)]
pub struct NodeRects {
    /// node rects
    graph_nodes: AHashMap<NodeId, Rect>,
    region_hull_cache: AHashMap<Uuid, CachedHull>,
}

#[derive(Debug, Clone)]
struct CachedHull {
    points: Vec<Pos2>,
    hash: u64,
}

impl NodeRects {
    /// Adds a node rect in screen space
    pub fn add_graph_space_rect(&mut self, node_id: NodeId, rect: Rect) {
        self.graph_nodes.insert(node_id, rect);
    }

    /// Calculates hull of the region in screen space using the provided viewport
    pub fn screen_space_hull(
        &mut self,
        region: &Uuid,
        region_graph: &RegionGraphData,
        viewport: &Viewport,
    ) -> Option<Vec<Pos2>> {
        let region_data = region_graph.region_data(region);
        let mut hasher = AHasher::default();
        let mut points = Vec::with_capacity(region_data.nodes.len() * 4);
        for node in &region_data.nodes {
            if let Some(rect) = self.graph_nodes.get(&node.node) {
                let rect = rect.expand(
                    node.separation as f32 * tweak!(7.5)
                        + if node.node == region_data.start_node
                            || node.node == region_data.end_node
                        {
                            tweak!(10.0)
                        } else {
                            tweak!(7.5)
                        },
                );

                let rect = graph_to_screen(&rect, viewport);

                if node.node == region_data.start_node {
                    points.push(rect.center_top());
                    points.push(rect.center_bottom());
                } else {
                    points.push(rect.left_top());
                    points.push(rect.left_bottom());
                }

                if node.node == region_data.end_node {
                    points.push(rect.center_top());
                    points.push(rect.center_bottom());
                } else {
                    points.push(rect.right_top());
                    points.push(rect.right_bottom());
                }

                for pos in points.iter().rev().take(4) {
                    OrderedFloat::from(pos.x).hash(&mut hasher);
                    OrderedFloat::from(pos.y).hash(&mut hasher);
                }
            }
        }
        if points.len() < 3 {
            return None;
        }

        let hash = hasher.finish();

        if let Some(cached) = self.region_hull_cache.get(region) {
            if cached.hash == hash {
                return Some(cached.points.clone());
            } else {
                // trace!(%region, cached.hash, hash, "Hull cache mismatch, recomputing");
            }
        }

        let mut hull = ConvexHull2D::with_data(&points);
        hull.compute();

        let mut points = Vec::with_capacity(hull.hulls.len());
        for idx in hull.hulls {
            points.push(hull.data[idx]);
        }

        self.region_hull_cache.insert(
            *region,
            CachedHull {
                points: points.clone(),
                hash,
            },
        );
        Some(points)
    }
}

fn graph_to_screen(rect: &Rect, viewport: &Viewport) -> Rect {
    Rect::from_min_max(
        viewport.graph_pos_to_screen(rect.min),
        viewport.graph_pos_to_screen(rect.max),
    )
}
