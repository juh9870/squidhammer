use emath::{Pos2, Vec2};
use strum::EnumIs;

pub mod convex_hull_2d;

pub mod minkowski;

const EPS: f32 = 1e-5;

/// Calculate the cross product of two vectors
pub fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x.mul_add(b.y, -(a.y * b.x))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIs)]
pub enum WindingDirection {
    Clockwise,
    CounterClockwise,
}

/// Calculate the winding direction of a convex polygon
///
/// Returns None if the winding direction cannot be determined (e.g. all points
/// are collinear)
pub fn convex_winding_direction(points: &[Pos2]) -> Option<WindingDirection> {
    assert!(points.len() > 1);
    let mut dx = (points[1] - points[0]).normalized();
    for i in 1..points.len() {
        let next_dx = (points[(i + 1) % points.len()] - points[i]).normalized();

        if vec_approx_eq(next_dx, Vec2::ZERO) {
            continue;
        }

        if vec_approx_eq(dx, Vec2::ZERO) {
            dx = next_dx;
            continue;
        }

        let cross = cross(dx, next_dx);

        // Cross product is significant enough to determine winding direction
        if cross.abs() > 0.015 {
            match cross.signum() {
                1.0 => return Some(WindingDirection::CounterClockwise),
                -1.0 => return Some(WindingDirection::Clockwise),
                _ => {
                    unreachable!("Cross product is not zero, but signum is zero")
                }
            }
        }
    }

    None
}

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < (a * EPS).max(EPS)
}

fn vec_approx_eq(a: Vec2, b: Vec2) -> bool {
    approx_eq(a.x, b.x) && approx_eq(a.y, b.y)
}
