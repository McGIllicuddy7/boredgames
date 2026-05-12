use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap, HashSet},
};

use rand::{random, seq::SliceRandom};
use raylib::{color::Color, math::Vector2, texture::Image};
use serde::de;

use crate::libgui::{Bounds, Point};

#[derive(Clone, Debug)]
pub struct Building {
    pub bounds: Bounds,
    pub rotation: f32,
}

#[derive(Clone, Debug)]
pub struct City {
    pub buildings: Vec<Building>,
    pub roads: Vec<Road>,
}

#[derive(Clone, Debug)]
pub struct Road {
    pub points: Vec<Point>,
}

pub fn generate_city(building_count: usize) -> City {
    let mut city = City {
        buildings: Vec::new(),
        roads: generate_roads(),
    };
    let mut points = points_near_roads(&city.roads, 5, 15);
    while let Some(_) = grow_city(&mut city, &mut points) {
        if city.buildings.len() >= building_count {
            break;
        }
    }
    city
}

pub fn grow_city(city: &mut City, points: &mut Vec<(Point, f32)>) -> Option<()> {
    println!("growing");
    points.shuffle(&mut rand::rng());
    let mut hit = false;
    'outer: while let Some((p, theta)) = points.pop() {
        for h in 5..=10 {
            for w in 5..=10 {
                let b = Building {
                    bounds: Bounds {
                        x: p.x - w / 2,
                        y: p.y - h / 2,
                        width: w,
                        height: h,
                    },
                    rotation: theta,
                };
                let mut collides = false;
                for j in &city.buildings {
                    if b.collides_with(j) {
                        collides = true;
                        break;
                    }
                }
                if !collides {
                    for j in &city.roads {
                        if b.collides_with_road(j) {
                            collides = true;
                            break;
                        }
                    }
                }
                if !collides {
                    println!("{:#?}", city.buildings.len());
                    city.buildings.push(b);
                    hit = true;
                    break 'outer;
                }
            }
        }
    }
    if hit {
        return Some(());
    } else {
        return None;
    }
}

pub fn distance_to_line_segmment(point: Point, start: Point, end: Point) -> i32 {
    let mut base = Vector2::new(start.x as f32, start.y as f32);
    let delta = Vector2::new((end.x - start.x) as f32, (end.y - start.y) as f32).normalized();
    let count = (Vector2::new((end.x - start.x) as f32, (end.y - start.y) as f32))
        .length()
        .ceil() as u32;
    let p = Vector2::new(point.x as f32, point.y as f32);
    let mut min_dist = base.distance_to(p);
    for _ in 0..count {
        base += delta;
        let tmp = base.distance_to(p);
        if tmp < min_dist {
            min_dist = tmp;
        }
    }
    min_dist as i32
}

pub fn points_near_roads(road_set: &[Road], min_dist: i32, max_dist: i32) -> Vec<(Point, f32)> {
    let mut out_set: HashSet<Point> = HashSet::new();
    let mut out_norms = HashMap::new();
    let mut count = 0;
    for i in road_set {
        for j in &i.points {
            for dy in -max_dist * 4..=max_dist * 4 {
                for dx in -max_dist * 4..=max_dist * 4 {
                    let p = Point {
                        x: j.x + dx,
                        y: j.y + dy,
                    };
                    if out_set.contains(&p) {
                        continue;
                    }
                    let (dist, angle) = i.distance_to_nearest_normal(p);
                    if dist >= min_dist && dist < max_dist {
                        out_set.insert(p);
                        out_norms.insert(p, angle);
                    }
                }
            }
        }
        println!("count:{}", count);
        count += 1;
    }
    let mut out = Vec::new();
    for i in &out_set {
        out.push((*i, *out_norms.get(i).unwrap()));
    }
    partialordsort(&mut out, |i, j| {
        ((i.0.x - 500) * (i.0.x - 500) + (i.0.y - 500) * (i.0.x - 500))
            .cmp(&((j.0.x - 500) * (j.0.x - 500) + (j.0.y - 500) * (j.0.y - 500)))
    });
    out
}

impl Road {
    pub fn distance_to(&self, p: Point) -> i32 {
        let p2 = Vector2::new(p.x as f32, p.y as f32);
        let mut min =
            Vector2::new(self.points[0].x as f32, self.points[0].y as f32).distance_to(p2) as i32;
        for i in 0..self.points.len() - 1 {
            let d = distance_to_line_segmment(p, self.points[i], self.points[i + 1]);
            if d < min {
                min = d;
            }
        }
        min as i32
    }
    pub fn distance_to_nearest_normal(&self, p: Point) -> (i32, f32) {
        let p2 = Vector2::new(p.x as f32, p.y as f32);
        let mut min =
            Vector2::new(self.points[0].x as f32, self.points[0].y as f32).distance_to(p2) as i32;
        let mut angle = 0.0;
        for i in 0..self.points.len() - 1 {
            let d = distance_to_line_segmment(p, self.points[i], self.points[i + 1]);
            if d < min {
                min = d;
                let delta = Vector2::new(
                    (self.points[i + 1].x - self.points[i].x) as f32,
                    (self.points[i + 1].y - self.points[i].y) as f32,
                );
                let g = delta.angle_to(Vector2::new(1.0, 0.0));
                angle = g;
            }
        }
        (min as i32, angle)
    }
}

impl Building {
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

    pub fn collides_with(&self, other: &Building) -> bool {
        let svs = self.vertices();
        let ovs = other.vertices();
        let snorms = self.normals();
        let onorms = other.normals();
        let normals = [
            snorms[0], snorms[1], snorms[2], snorms[3], snorms[4], snorms[5], snorms[6], snorms[7],
            onorms[0], onorms[1], onorms[2], onorms[3], onorms[4], onorms[5], onorms[6], onorms[7],
        ];
        for i in normals {
            let mut smin = svs[0].dot(i);
            let mut smax = svs[0].dot(i);
            let mut omin = ovs[0].dot(i);
            let mut omax = ovs[0].dot(i);
            for j in svs {
                let v = i.dot(j);
                if v > smax {
                    smax = v;
                }
                if v < smin {
                    smin = v;
                }
            }
            for j in ovs {
                let v = i.dot(j);
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
                return false;
            }
        }
        true
    }

    pub fn collides_with_road(&self, road: &Road) -> bool {
        for j in self.vertices() {
            let d = road.distance_to(Point {
                x: j.x as i32,
                y: j.y as i32,
            });
            if d < 5 {
                return true;
            }
        }
        false
    }
}

pub fn generate_roads() -> Vec<Road> {
    let mut grid: Vec<Vec<Point>> = Vec::new();
    for y in 0..50 {
        let mut list = Vec::new();
        for x in 0..50 {
            list.push(Point {
                x: x * 20,
                y: y * 20,
            });
        }
        grid.push(list);
    }
    let mut v = Vec::new();
    for i in &mut grid {
        for j in i {
            j.x += (random::<u32>() % 10) as i32 - 5;
            j.y += (random::<u32>() % 10) as i32 - 5;
        }
    }
    for dx in 0..50 {
        let mut road_list = Vec::new();
        for dy in 0..50 {
            road_list.push(grid[dy][dx]);
        }
        v.push(Road { points: road_list });
    }
    for dy in 0..50 {
        let mut road_list = Vec::new();
        for dx in 0..50 {
            road_list.push(grid[dy][dx]);
        }
        v.push(Road { points: road_list });
    }
    v
}

impl City {
    pub fn render(&self) {
        let mut out = Image::gen_image_color(1000, 1000, Color::WHITE);
        for i in &self.roads {
            for j in 0..i.points.len() - 1 {
                out.draw_line(
                    i.points[j].x,
                    i.points[j].y,
                    i.points[j + 1].x,
                    i.points[j + 1].y,
                    Color::BLACK,
                );
            }
        }
        for j in &self.buildings {
            let points = j.vertices();
            let p0 = points[0];
            let p1 = points[1];
            let p2 = points[2];
            let p3 = points[3];
            out.draw_line(
                p0.x as i32,
                p0.y as i32,
                p1.x as i32,
                p1.y as i32,
                Color::BLACK,
            );
            out.draw_line(
                p0.x as i32,
                p0.y as i32,
                p2.x as i32,
                p2.y as i32,
                Color::BLACK,
            );
            out.draw_line(
                p3.x as i32,
                p3.y as i32,
                p1.x as i32,
                p1.y as i32,
                Color::BLACK,
            );
            out.draw_line(
                p3.x as i32,
                p3.y as i32,
                p2.x as i32,
                p2.y as i32,
                Color::BLACK,
            );
        }
    }
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
