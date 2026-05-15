use std::cmp::Ordering;

use raylib::math::Vector2;

use crate::libgui::{Bounds, Point};

pub mod buildings;
pub mod city;

//https://www.geeksforgeeks.org/dsa/minimum-distance-from-a-point-to-the-line-segment-using-vectors/
pub fn distance_to_line_segmment(point: Point, start: Point, end: Point) -> i32 {
    let e = point.as_vec2();
    let a = start.as_vec2();
    let b = end.as_vec2();
    let ab = end.as_vec2() - start.as_vec2();

    // vector BP
    let be = point.as_vec2() - end.as_vec2();
    // BE.F = E.F - B.F;
    //BE.S = E.S - B.S;

    // vector AP
    let ae = point.as_vec2() - start.as_vec2();
    //AE.F = E.F - A.F,
    //AE.S = E.S - A.S;

    // Calculating the dot product
    let ab_be = ab.x * be.x + ab.y * be.y;
    let ab_ae = ab.x * ae.x + ab.y * ae.y;

    // Minimum distance from
    // point E to the line segment
    let mut req_ans = 0.0;

    // Case 1
    if ab_be > 0. {
        // Finding the magnitude
        let x = e.x - b.x;
        let y = e.y - b.y;
        req_ans = (x * x + y * y).sqrt();
    }
    // Case 2
    else if ab_ae < 0. {
        let x = e.x - a.x;
        let y = e.y - a.y;
        req_ans = (x * x + y * y).sqrt();
    }
    // Case 3
    else {
        // Finding the perpendicular distance
        let x1 = ab.x;
        let y1 = ab.y;
        let x2 = ae.x;
        let y2 = ae.y;
        let md = (x1 * x1 + y1 * y1).sqrt();
        req_ans = (x1 * y2 - y1 * x2).abs() / md;
    }
    req_ans as i32
}

//https://stackoverflow.com/questions/78588965/how-to-sort-a-vector-in-rust-that-only-has-partial-ordering
pub fn partialordsort<T>(mut items: &mut [T], mut cmp: impl FnMut(&T, &T) -> Ordering) {
    let mut presorted = 1;
    while items.len() > presorted {
        'make_start_min: loop {
            for i in presorted..items.len() {
                let ordering = cmp(&items[0], &items[i]);
                if ordering == Ordering::Greater {
                    items.swap(presorted, i);
                    items[0..presorted + 1].rotate_right(1);
                    presorted += 1;
                    continue 'make_start_min;
                }
            }

            break;
        }

        presorted = usize::max(presorted - 1, 1);
        items = &mut items[1..];
    }
}

pub fn distance_between_line_segments(
    ls0_start: Point,
    ls0_end: Point,
    ls1_start: Point,
    ls1_end: Point,
) -> i32 {
    let (start, end, v0, v1) = if (Vector2::new(ls0_start.x as f32, ls0_start.y as f32)
        .distance_to(Vector2::new(ls0_end.x as f32, ls0_end.y as f32)))
        < (Vector2::new(ls1_start.x as f32, ls1_start.y as f32)
            .distance_to(Vector2::new(ls1_end.x as f32, ls1_end.y as f32)))
    {
        (
            Vector2::new(ls0_start.x as f32, ls0_start.y as f32),
            Vector2::new(ls0_end.x as f32, ls0_end.y as f32),
            ls1_start,
            ls1_end,
        )
    } else {
        (
            Vector2::new(ls1_start.x as f32, ls1_start.y as f32),
            Vector2::new(ls1_end.x as f32, ls1_end.y as f32),
            ls0_start,
            ls0_end,
        )
    };
    let mut base = Vector2::new(start.x, start.y);
    let delta = Vector2::new(end.x - start.x, end.y - start.y).normalized();
    let count = (Vector2::new(end.x - start.x, end.y - start.y))
        .length()
        .ceil() as u32;
    let mut min_dist = distance_to_line_segmment(
        Point {
            x: base.x as i32,
            y: base.y as i32,
        },
        v0,
        v1,
    );
    for _ in 0..count {
        base += delta;
        let tmp = distance_to_line_segmment(
            Point {
                x: base.x as i32,
                y: base.y as i32,
            },
            v0,
            v1,
        );
        if tmp < min_dist {
            min_dist = tmp;
        }
    }
    min_dist
}

pub fn collision_seperating_axis_theorem_points(
    set_1: &[Point],
    set_2: &[Point],
) -> Option<Vector2> {
    let mut c1 = Vector2::zero();
    let mut c2 = Vector2::zero();
    for i in set_1 {
        c1 += i.as_vec2();
    }
    for i in set_2 {
        c2 += i.as_vec2();
    }
    let c1 = c1 / (set_1.len() as f32);
    let c2 = c2 / (set_2.len() as f32);
    let mut normals: Vec<Vector2> = Vec::new();
    normals.reserve_exact(set_1.len() * 2 + set_2.len() * 2);
    for i in 0..set_1.len() {
        let j = (i + 1) % set_1.len();
        let v0 = set_1[i].as_vec2();
        let v1 = set_1[j].as_vec2();
        let dir = (v1 - v0).normalized();
        let cent = (v1 + v0) / 2.;
        let dcent = (cent - c1).normalized();
        let n1 = dir.rotated(std::f32::consts::PI / 2.);
        let norm = if n1.dot(dcent) > 0.0 { n1 } else { -n1 };
        let n0 = (v0 - c1).normalized();
        normals.push(norm);
        normals.push(n0);
    }
    for i in 0..set_2.len() {
        let j = (i + 1) % set_2.len();
        let v0 = set_2[i].as_vec2();
        let v1 = set_2[j].as_vec2();
        let dir = (v1 - v0).normalized();
        let cent = (v1 + v0) / 2.;
        let dcent = (cent - c2).normalized();
        let n1 = dir.rotated(std::f32::consts::PI / 2.);
        let norm = if n1.dot(dcent) > 0.0 { n1 } else { -n1 };
        let n0 = (v0 - c2).normalized();
        normals.push(norm);
        normals.push(n0);
    }
    let mut min_delta: f32 = 10000000000.0;
    let mut min_normal = normals[0];
    for i in normals {
        let mut smin = set_1[0].as_vec2().dot(i);
        let mut smax = set_1[0].as_vec2().dot(i);
        let mut omin = set_2[0].as_vec2().dot(i);
        let mut omax = set_2[0].as_vec2().dot(i);
        for j in set_1 {
            let v = i.dot(j.as_vec2());
            if v > smax {
                smax = v;
            }
            if v < smin {
                smin = v;
            }
        }
        for j in set_2 {
            let v = i.dot(j.as_vec2());
            if v > omax {
                omax = v;
            }
            if v < omin {
                omin = v;
            }
        }

        if (smin < omin && smax < omin && smin < omax && smax < omax)
            || (omin < smin && omax < omin && omin < omax && omax < smax)
        {
            return None;
        }
        let delta_1 = (omin - smax).abs();
        let delta_2 = (smin - omax).abs();
        let delta_3 = (smax - omax).abs();
        let delta_4 = (smin - omin).abs();
        let delta = min(min(delta_1, delta_2), min(delta_3, delta_4));
        if delta < min_delta {
            min_delta = delta;
            min_normal = i;
        }
    }
    Some(min_normal)
}

pub fn collision_seperating_axis_theorem(set_1: &[Vector2], set_2: &[Vector2]) -> Option<Vector2> {
    let mut c1 = Vector2::zero();
    let mut c2 = Vector2::zero();
    for i in set_1 {
        c1 += *i;
    }
    for i in set_2 {
        c2 += *i;
    }
    let c1 = c1 / (set_1.len() as f32);
    let c2 = c2 / (set_2.len() as f32);
    let mut normals: Vec<Vector2> = Vec::new();
    normals.reserve_exact(set_1.len() * 2 + set_2.len() * 2);
    for i in 0..set_1.len() {
        let j = (i + 1) % set_1.len();
        let v0 = set_1[i];
        let v1 = set_1[j];
        let dir = (v1 - v0).normalized();
        let cent = (v1 + v0) / 2.;
        let dcent = (cent - c1).normalized();
        let n1 = dir.rotated(std::f32::consts::PI / 2.);
        let norm = if n1.dot(dcent) > 0.0 { n1 } else { -n1 };
        let n0 = (v0 - c1).normalized();
        normals.push(norm);
        normals.push(n0);
    }
    for i in 0..set_2.len() {
        let j = (i + 1) % set_2.len();
        let v0 = set_2[i];
        let v1 = set_2[j];
        let dir = (v1 - v0).normalized();
        let cent = (v1 + v0) / 2.;
        let dcent = (cent - c2).normalized();
        let n1 = dir.rotated(std::f32::consts::PI / 2.);
        let norm = if n1.dot(dcent) > 0.0 { n1 } else { -n1 };
        let n0 = (v0 - c2).normalized();
        normals.push(norm);
        normals.push(n0);
    }
    let mut min_delta: f32 = 10000000000.0;
    let mut min_normal = normals[0];
    for i in normals {
        let mut smin = set_1[0].dot(i);
        let mut smax = set_1[0].dot(i);
        let mut omin = set_2[0].dot(i);
        let mut omax = set_2[0].dot(i);
        for j in set_1 {
            let v = i.dot(*j);
            if v > smax {
                smax = v;
            }
            if v < smin {
                smin = v;
            }
        }
        for j in set_2 {
            let v = i.dot(*j);
            if v > omax {
                omax = v;
            }
            if v < omin {
                omin = v;
            }
        }

        if (smin < omin && smax < omin && smin < omax && smax < omax)
            || (omin < smin && omax < omin && omin < omax && omax < smax)
        {
            return None;
        }
        let delta_1 = (omin - smax).abs();
        let delta_2 = (smin - omax).abs();
        let delta_3 = (smax - omax).abs();
        let delta_4 = (smin - omin).abs();
        let delta = min(min(delta_1, delta_2), min(delta_3, delta_4));
        if delta < min_delta {
            min_delta = delta;
            min_normal = i;
        }
    }
    Some(min_normal)
}
pub fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

pub fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

#[derive(Clone, Debug, Copy)]
pub struct Boundary {
    pub bounds: Bounds,
    pub rotation: f32,
}
impl Boundary {
    pub fn vertices(&self) -> [Vector2; 4] {
        let mut out = [
            Vector2::new(
                -self.bounds.width as f32 / 2.,
                -self.bounds.height as f32 / 2.,
            ),
            Vector2::new(
                self.bounds.width as f32 / 2.,
                -self.bounds.height as f32 / 2.,
            ),
            Vector2::new(
                -self.bounds.width as f32 / 2.,
                self.bounds.height as f32 / 2.,
            ),
            Vector2::new(
                self.bounds.width as f32 / 2.,
                self.bounds.height as f32 / 2.,
            ),
        ];
        let center = Vector2::new(self.bounds.x as f32, self.bounds.y as f32)
            + Vector2::new(
                self.bounds.width as f32 / 2.0,
                self.bounds.height as f32 / 2.0,
            );
        for i in &mut out {
            *i = i.rotated(self.rotation) + center;
        }
        out
    }

    pub fn normals(&self) -> [Vector2; 8] {
        let mut out = [
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(-1., 0.0),
            Vector2::new(0.0, -1.0),
            Vector2::new(1.0, 1.0).normalized(),
            Vector2::new(1.0, -1.0).normalized(),
            Vector2::new(-1.0, 1.0).normalized(),
            Vector2::new(1.0, -1.0).normalized(),
        ];
        for i in &mut out {
            *i = i.rotated(self.rotation);
        }
        out
    }
    pub fn check_collision(&self, other: &Self) -> bool {
        collision_seperating_axis_theorem(&self.vertices(), &other.vertices()).is_some()
    }
    pub fn distance_to_line(&self, start: Point, end: Point) -> i32 {
        let vs = self.vertices();
        let p0 = Point::from_vec2(vs[0]);
        let p1 = Point::from_vec2(vs[1]);
        let p2 = Point::from_vec2(vs[2]);
        let p3 = Point::from_vec2(vs[3]);
        let d0 = distance_between_line_segments(p0, p1, start, end);
        let d1 = distance_between_line_segments(p0, p2, start, end);
        let d2 = distance_between_line_segments(p3, p1, start, end);
        let d3 = distance_between_line_segments(p3, p2, start, end);
        min(min(d0, d1), min(d2, d3))
    }
}

pub struct Queue<T> {
    values: Vec<T>,
    compare: Box<dyn Fn(&T, &T) -> std::cmp::Ordering>,
}
impl<T: PartialOrd> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PartialOrd> Queue<T> {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            compare: Box::new(|a, b| {
                if let Some(cmp) = a.partial_cmp(b) {
                    cmp
                } else {
                    std::cmp::Ordering::Greater
                }
            }),
        }
    }
}

impl<T> Queue<T> {
    pub fn new_cmp(comp: impl Fn(&T, &T) -> std::cmp::Ordering + 'static) -> Self {
        Self {
            values: Vec::new(),
            compare: Box::new(comp),
        }
    }

    pub fn insert(&mut self, value: T) {
        if let Some(idx) = (0..self.values.len()).next() {
            if (self.compare)(&value, &self.values[idx]) == std::cmp::Ordering::Less {
                self.values.insert(idx, value);
            }
            return;
        }
        self.values.push(value);
    }
    pub fn pop(&mut self) -> Option<T> {
        self.values.pop()
    }
}
