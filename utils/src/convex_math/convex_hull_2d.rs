// adapted from https://github.com/Donvlouss/f3l/blob/main/crates/f3l_surface/src/convex_hull/convex_hull_2d.rs

use crate::convex_math::EPS;
use num_traits::Float;
use std::{borrow::Cow, ops::Index};

/// Convex Hull Trait.
pub trait Convex<'a, P> {
    // fn new(data: &'a Vec<P>) -> Self;
    fn new() -> Self;
    fn with_data(data: &'a [P]) -> Self;
    fn set_data(&mut self, data: &'a [P]);
    fn compute(&mut self);
    // fn compute(&mut self, data: &Vec<P>);
}

/// Convex Hull of 2d data.
/// A `QuickHull` implement.
#[derive(Debug, Clone, Default)]
pub struct ConvexHull2D<'a, T: Float, P>
where
    P: Into<[T; 2]> + Clone + Copy + Send + Sync + Index<usize, Output = T>,
{
    pub data: Cow<'a, [P]>,
    pub hulls: Vec<usize>,
}

impl<'a, T: Float, P> ConvexHull2D<'a, T, P>
where
    P: Into<[T; 2]> + Clone + Copy + Send + Sync + Index<usize, Output = T>,
{
    /// Return line model as `[a, b, c]` `a`x + `b`y + `c` = 0.
    #[inline]
    fn generate_line(id: &(P, P)) -> [T; 3] {
        if (id.0[0] - id.1[0]).abs() <= T::from(EPS).unwrap() {
            return [T::one(), T::zero(), -id.0[0]];
        }
        if (id.0[1] - id.1[1]).abs() <= T::from(EPS).unwrap() {
            return [T::zero(), T::one(), -id.0[1]];
        }

        let m = (id.1[1] - id.0[1]) / (id.1[0] - id.0[0]);
        [m, -T::one(), id.0[1] - id.0[0] * m]
    }

    /// Return distance of `p` to `line`.
    #[inline]
    fn distance_slice(line: &[T; 3], p: &[T; 2]) -> T {
        line[0] * p[0] + line[1] * p[1] + line[2]
    }

    /// Split points to outside or inside by line and signed.
    fn split_data(&self, line: &[T; 3], points: &[usize], signed: bool, outside: &mut Vec<usize>) {
        *outside = points
            .iter()
            .filter_map(|&i| {
                let d = Self::distance_slice(line, &self.data[i].into())
                    * if signed { -T::one() } else { T::one() };
                if d < T::zero() {
                    Some(i)
                } else {
                    None
                }
            })
            .collect::<Vec<usize>>()
    }

    /// QuickHull implement.
    ///
    /// 1. Find the farthest point of line and outside points.
    /// 2. Insert the farthest point to edge_start and edge_end.
    /// 3. Split data to right and left of two edges.
    /// 4. recursive finding until no outside data.
    fn compute_recursive(&self, ids: &[usize], edge: &[usize; 2], hulls: &mut Vec<usize>) {
        if ids.is_empty() {
            // no outside, is convex hull already.
            return;
        } else if ids.len() == 1 {
            // only one outside, set it to hull directly.
            hulls.insert(edge[1], ids[0]);
            return;
        }

        let [start, end] = *edge;

        let (head, tail) = (self.data[hulls[edge[0]]], self.data[hulls[edge[1]]]);

        let line = Self::generate_line(&(head, tail));

        let mut farthest = 0_usize;
        let mut farthest_value = T::zero();
        ids.iter().for_each(|&i| {
            let d = Self::distance_slice(&line, &self.data[i].into()).abs();
            if d > farthest_value {
                farthest_value = d;
                farthest = i;
            }
        });
        if farthest_value <= T::from(EPS).unwrap() {
            return;
        }
        hulls.insert(end, farthest);
        let mid = self.data[farthest];

        let center = [
            (head[0] + tail[0] + mid[0]) / T::from(3f32).unwrap(),
            (head[1] + tail[1] + mid[1]) / T::from(3f32).unwrap(),
        ];

        let splits = [
            Self::generate_line(&(head, mid)),
            Self::generate_line(&(mid, tail)),
        ]
        .into_iter()
        .map(|line| {
            let sign = Self::distance_slice(&line, &center) < T::zero();
            let mut outside = Vec::with_capacity(ids.len());
            self.split_data(&line, ids, sign, &mut outside);
            outside
        })
        .collect::<Vec<_>>();

        self.compute_recursive(&splits[1], &[end, end + 1], hulls);
        self.compute_recursive(&splits[0], &[start, end], hulls);
    }
}

impl<'a, T: Float, P> Convex<'a, P> for ConvexHull2D<'a, T, P>
where
    P: Into<[T; 2]> + Clone + Copy + Send + Sync + Index<usize, Output = T>,
{
    fn new() -> Self {
        Self {
            data: Cow::Owned(vec![]),
            hulls: vec![],
        }
    }

    fn with_data(data: &'a [P]) -> Self {
        Self {
            data: Cow::Borrowed(data),
            hulls: vec![],
        }
    }

    fn set_data(&mut self, data: &'a [P]) {
        self.data = Cow::Borrowed(data);
    }

    /// 1. Find min max of xy, got 4 ids
    /// 2. Find longest distance between ids as edge.
    /// 3. Split data to upper and lower data, compute recursive to insert hulls.
    fn compute(&mut self) {
        let data = &self.data;
        assert!(data.len() >= 3);

        let mut min_id = [0usize; 2];
        let mut max_id = [0usize; 2];
        let mut min_v = [data[0][0]; 2];
        let mut max_v = [data[0][1]; 2];
        (0..data.len()).for_each(|i| {
            (0..2).for_each(|ii| {
                if data[i][ii] < min_v[ii] {
                    min_v[ii] = data[i][ii];
                    min_id[ii] = i;
                }
                if data[i][ii] > max_v[ii] {
                    max_v[ii] = data[i][ii];
                    max_id[ii] = i;
                }
            });
        });
        let largest = if (max_v[0] - min_v[0]) > (max_v[1] - min_v[1]) {
            0
        } else {
            1
        };

        let (p0, p1) = (data[min_id[largest]], data[max_id[largest]]);
        let line = Self::generate_line(&(p0, p1));

        let mut side_a = Vec::with_capacity(data.len());
        let mut side_b = Vec::with_capacity(data.len());
        (0..data.len()).for_each(|i| {
            let d = Self::distance_slice(&line, &data[i].into());
            if d < T::zero() {
                side_a.push(i);
            } else if d > T::zero() {
                side_b.push(i);
            }
        });

        let mut hull_a = Vec::with_capacity(data.len());
        let mut hull_b = Vec::with_capacity(data.len());

        hull_a.push(min_id[largest]);
        hull_a.push(max_id[largest]);

        hull_b.push(max_id[largest]);
        hull_b.push(min_id[largest]);

        self.compute_recursive(&side_a, &[0, 1], &mut hull_a);
        self.compute_recursive(&side_b, &[0, 1], &mut hull_b);

        let nb_hull_b = hull_b.len();
        let hull_b = &mut hull_b[1..nb_hull_b - 1].to_owned();
        if !hull_b.is_empty() {
            hull_a.append(hull_b);
        }
        self.hulls = hull_a;
    }
}

#[test]
fn convex_hull_2d() {
    let points = vec![
        [0f32, 0.],
        [3., -1.],
        [6., 0.],
        [5., 3.],
        [3., 4.],
        [1., 3.],
        [0.5, 2.],
    ];
    let mut cvh = ConvexHull2D::with_data(&points);
    cvh.compute();
    assert_eq!(cvh.hulls, [0_usize, 1, 2, 3, 4, 5, 6]);
}
