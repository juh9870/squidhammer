use ahash::{AHashMap, AHasher};
use dbe_backend::graph::region::region_graph::RegionGraphData;
use egui::emath::OrderedFloat;
use egui::{Pos2, Rect};
use egui_snarl::ui::Viewport;
use egui_snarl::NodeId;
use inline_tweak::tweak;
use std::hash::{Hash, Hasher};
use utils::convex_math::convex_hull_2d::{Convex, ConvexHull2D};
use utils::convex_math::convex_winding_direction;
use utils::convex_math::minkowski::minkowski;
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
                            tweak!(2.5)
                        } else {
                            tweak!(0.0)
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

        let circle = poly_circle(tweak!(7.5) * viewport.scale);

        if !convex_winding_direction(&points).is_some_and(|w| w.is_counter_clockwise()) {
            points.reverse();
        }

        debug_assert_eq!(
            convex_winding_direction(&points),
            convex_winding_direction(&circle)
        );

        let mut shape = minkowski(points, circle);

        shape.reverse();

        self.region_hull_cache.insert(
            *region,
            CachedHull {
                points: shape.clone(),
                hash,
            },
        );
        Some(shape)
    }
}

fn graph_to_screen(rect: &Rect, viewport: &Viewport) -> Rect {
    Rect::from_min_max(
        viewport.graph_pos_to_screen(rect.min),
        viewport.graph_pos_to_screen(rect.max),
    )
}

fn poly_circle(radius: f32) -> Vec<Pos2> {
    const PI: f32 = std::f32::consts::PI;

    let perimeter = 2.0 * PI * radius;
    let vertices = ((perimeter * tweak!(0.15)).round() as usize).max(tweak!(12));

    let mut points = Vec::with_capacity(vertices);
    for i in 0..vertices {
        let angle = 2.0 * PI * i as f32 / vertices as f32;
        points.push(Pos2::new(angle.cos(), angle.sin()) * radius);
    }

    points
}
