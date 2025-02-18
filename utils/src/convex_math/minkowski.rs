//! ported from https://cp-algorithms.com/geometry/minkowski.html#implementation

use crate::convex_math::{convex_winding_direction, cross, WindingDirection};
use emath::Pos2;

/// Calculate the Minkowski sum of two convex polygons
///
/// The input polygons must be convex and have vertices in counter-clockwise order.
pub fn minkowski(mut p: Vec<Pos2>, mut q: Vec<Pos2>) -> Vec<Pos2> {
    // check winding direction but only when debug_assertions are enabled
    debug_assert_eq!(
        convex_winding_direction(&p),
        Some(WindingDirection::CounterClockwise)
    );
    debug_assert_eq!(
        convex_winding_direction(&q),
        Some(WindingDirection::CounterClockwise)
    );

    // the first vertex must be the lowest
    reorder_polygon(&mut p);
    reorder_polygon(&mut q);
    // we must ensure cyclic indexing
    p.push(p[0]);
    p.push(p[1]);
    q.push(q[0]);
    q.push(q[1]);
    // main part
    let mut result = vec![];
    let mut i = 0;
    let mut j = 0;
    while i < p.len() - 2 || j < q.len() - 2 {
        result.push(p[i] + q[j].to_vec2());
        let cross = cross(
            (p[i + 1] - p[i].to_vec2()).to_vec2(),
            (q[j + 1] - q[j].to_vec2()).to_vec2(),
        );
        if cross >= 0.0 && i < p.len() - 2 {
            i += 1;
        }
        if cross <= 0.0 && j < q.len() - 2 {
            j += 1;
        }

        #[cfg(debug_assertions)]
        assert!(result.len() <= 1000000, "Infinite loop")
    }

    result
}

fn reorder_polygon(points: &mut [Pos2]) {
    let mut pos = 0;
    for i in 1..points.len() {
        if points[i].y < points[pos].y
            || (points[i].y == points[pos].y && points[i].x < points[pos].x)
        {
            pos = i
        }
    }

    points.rotate_left(pos)
}
