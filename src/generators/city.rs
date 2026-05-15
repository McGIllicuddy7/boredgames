use core::f32;
use std::collections::VecDeque;

use rand::{random, random_bool};
use raylib::{color::Color, texture::Image};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::de;

#[allow(unused)]
use super::*;
#[derive(Clone, Debug)]
pub struct Building {
    pub bounds: Boundary,
}
#[derive(Clone, Debug)]
pub struct Road {
    pub points: Vec<Point>,
}

#[derive(Clone, Debug)]
pub struct City {
    pub buildings: Vec<Building>,
    pub roads: Vec<Road>,
}
pub fn create_city(building_count: usize) -> City {
    let start_road = Road {
        points: vec![Point { x: 450, y: 500 }, Point { x: 550, y: 500 }],
    };
    let mut city = City {
        buildings: Vec::new(),
        roads: Vec::new(),
    };
    city.roads.push(start_road);
    let mut point_queue = set_up_roads(&mut city);
    while grow_city(&mut city, &mut point_queue, 2, 20).is_some() {
        println!("{:#?}", city.buildings.len());
        if city.buildings.len() >= building_count {
            return city;
        }
    }
    println!("failed");
    city
}

pub fn grow_city(
    city: &mut City,
    queue: &mut Vec<(Point, f32)>,
    _min_dist: i32,
    _max_dist: i32,
) -> Option<()> {
    while let Some((p, theta)) = queue.pop() {
        let max_h = 10 + (random::<u64>() % 6) as i32;
        for height in (8..=max_h).rev() {
            for dw in -1..=1 {
                let width = height + dw;
                let b = Building {
                    bounds: Boundary {
                        bounds: Bounds {
                            x: p.x,
                            y: p.y,
                            width,
                            height,
                        },
                        rotation: theta,
                    },
                };
                if city.can_place_building(&b) {
                    city.buildings.push(b);
                    return Some(());
                }
            }
        }
    }
    None
}

pub fn set_up_roads(city: &mut City) -> Vec<(Point, f32)> {
    fn random_point() -> Point {
        let x = (random::<u64>() % 1000) as i32;
        let y = (random::<u64>() % 1000) as i32;
        Point { x, y }
    }
    fn next_point(last: Point, second_to_last: Point) -> Point {
        let delta = (last.as_vec2() - second_to_last.as_vec2()).normalized();
        let distance = (random::<u64>() % 24 + 16) as f32;
        let dtheta = ((random::<u64>() % 64) as i32 - 32) as f32 / (64.);
        let new_point = last.as_vec2() + delta.rotated(dtheta) * distance;
        Point::from_vec2(new_point)
    }
    //if bool true push_back
    fn next_point_from_list(list: &VecDeque<Point>) -> (bool, Point) {
        let should_go = list[0].as_vec2().distance_to(Vector2::new(500., 500.))
            < list[list.len() - 1]
                .as_vec2()
                .distance_to(Vector2::new(500., 500.));
        if (should_go && random_bool(0.9)) || (random_bool(0.35)) {
            let p0 = list[1];
            let p1 = list[0];
            let p2 = next_point(p1, p0);
            (false, p2)
        } else {
            let p0 = list[list.len() - 2];
            let p1 = list[list.len() - 1];
            let p2 = next_point(p1, p0);
            (true, p2)
        }
    }
    let mut point_queue = Vec::new();
    'generate: for i in 0..=25 {
        let mut v = VecDeque::new();
        let p0 = if i == 0 || random_bool(0.8) {
            let mut fs = 0;
            'l0: loop {
                let tmp = random_point();
                for j in &city.roads {
                    for k in &j.points {
                        if k.as_vec2().distance_to(tmp.as_vec2()) < 10. {
                            fs += 1;
                            if fs > 100 {
                                break 'generate;
                            }
                            continue 'l0;
                        }
                    }
                }
                break tmp;
            }
        } else {
            let idx = random::<u64>() as usize % city.roads.len();
            let idx2 = random::<u64>() as usize % (city.roads[idx].points.len());
            city.roads[idx].points[idx2]
        };
        let p1 = {
            let delta = Vector2::new(1.0, 0.0);
            let distance = (random::<u64>() % 24 + 40) as f32;
            let dtheta = ((random::<u64>() % 628) as i32) as f32 / (100.);
            let new_point = p0.as_vec2() + delta.rotated(dtheta) * distance;
            Point::from_vec2(new_point)
        };
        v.push_back(p0);
        v.push_back(p1);
        let mut dist = p1.as_vec2().distance_to(p0.as_vec2());
        let max_dist = (random::<u64>() % 300 + 300) as f32;
        let mut fs = 0;
        let mut last_close = false;
        'lp: while dist < max_dist {
            let mut close = false;
            let (back, mut p) = next_point_from_list(&v);
            'it: for j in &city.roads {
                for k in &j.points {
                    if k.as_vec2().distance_to(p.as_vec2()) < 10. {
                        p = *k;
                        break 'it;
                    }
                }
                for k in 0..j.points.len() - 1 {
                    let last = if back { v[v.len() - 1] } else { v[0] };
                    let d = distance_between_line_segments(last, p, j.points[k], j.points[k + 1]);
                    if d < 10 {
                        close = true;
                        break;
                    }
                }
            }
            if last_close && close {
                if back {
                    v.pop_back();
                } else {
                    v.pop_front();
                }
                if v.len() < 2 {
                    continue 'generate;
                }
                fs += 1;
                if fs >= 1000 {
                    break 'lp;
                } else {
                    continue 'lp;
                }
            } else {
                last_close = close;
            }
            let last = if back {
                let tmp = v[v.len() - 1];
                v.push_back(p);
                tmp
            } else {
                let tmp = v[0];
                v.push_front(p);
                tmp
            };
            let d = last.as_vec2().distance_to(p.as_vec2());
            dist += d;
            fs += 1;
            if fs >= 1000 {
                break;
            }
        }
        city.roads.push(Road { points: v.into() });
        println!("{}", i);
    }
    let mut pairs = Vec::new();
    let mut fs = 0;
    while pairs.len() < 20 {
        fs += 1;
        if fs > 1000 {
            break;
        }
        let s = random::<u64>() as usize % city.roads.len();
        let s2 = random::<u64>() as usize % city.roads.len();
        if s == s2 {
            continue;
        }
        let idx1 = (random::<u64>() as usize) % city.roads[s].points.len();
        let idx2 = (random::<u64>() as usize) % city.roads[s2].points.len();
        let p0 = city.roads[s].points[idx1];
        let p1 = city.roads[s2].points[idx2];
        if p0.as_vec2().distance_to(p1.as_vec2()) > 8000.
            || p0.as_vec2().distance_to(p1.as_vec2()) < 300.0
        {
            continue;
        }
        if p0 == p1 {
            continue;
        } else {
            pairs.push((p0, p1));
        }
    }
    for i in pairs {
        let mut points = Vec::new();
        let p0 = i.0.as_vec2();
        let p1 = i.1.as_vec2();
        let count = (p0.distance_to(p1) / 16.).floor() as i32;
        let delta = (p1 - p0).normalized() * 16.0;
        let mut current = p0;
        for _ in 0..count {
            points.push(Point::from_vec2(current));
            current += delta;
        }
        if points.len() < 2 {
            continue;
        }
        let r = Road { points };
        city.roads.push(r);
    }
    let mut pairs = Vec::new();
    fs = 0;
    while pairs.len() < 40 {
        fs += 1;
        if fs > 1000 {
            break;
        }
        let s = random::<u64>() as usize % city.roads.len();
        let s2 = random::<u64>() as usize % city.roads.len();
        if s == s2 {
            continue;
        }
        let idx1 = (random::<u64>() as usize) % city.roads[s].points.len();
        let idx2 = (random::<u64>() as usize) % city.roads[s2].points.len();
        let p0 = city.roads[s].points[idx1];
        let p1 = city.roads[s2].points[idx2];
        if p0.as_vec2().distance_to(p1.as_vec2()) > 300.
            || p0.as_vec2().distance_to(p1.as_vec2()) < 10.
        {
            continue;
        }
        if p0 == p1 {
            continue;
        } else {
            pairs.push((p0, p1));
        }
    }
    for i in pairs {
        let mut points = Vec::new();
        let p0 = i.0.as_vec2();
        let p1 = i.1.as_vec2();
        let count = (p0.distance_to(p1) / 16.).floor() as i32;
        let delta = (p1 - p0).normalized() * 16.0;
        let mut current = p0;
        for _ in 0..count {
            points.push(Point::from_vec2(current));
            current += delta;
        }
        if points.len() < 2 {
            continue;
        }
        let r = Road { points };
        city.roads.push(r);
    }
    let mut spikes: Vec<(Point, Point, Point)> = Vec::new();
    for (idx, i) in city.roads.iter().enumerate() {
        for j in 1..i.points.len() - 1 {
            if random_bool(
                (i.points[j].as_vec2().distance_to(Vector2::new(500., 500.)) / 500.)
                    .sqrt()
                    .clamp(0.1, 0.4) as f64,
            ) {
                let p0 = i.points[j - 1];
                let p1 = i.points[j];
                let p2 = i.points[j + 1];
                let delta_1 = (p0.as_vec2() - p1.as_vec2()).normalized().rotated(90.0);
                let delta_2 = (p2.as_vec2() - p1.as_vec2()).normalized().rotated(90.0);
                let delta2 = if delta_2.dot(delta_1) < 0.0 {
                    -delta_2
                } else {
                    delta_2
                };
                let delta = (delta2 + delta_1) / 2.0;
                let scale_1 = (random::<u64>() % 16 + 8) as f32;
                let s1 = Point::from_vec2(p1.as_vec2() + delta * scale_1 / 2.0);
                let s2 = Point::from_vec2(p1.as_vec2() + delta * scale_1);
                let s0 = [p1, s1, s2];
                let mut hit_count = 0;
                for l in s0 {
                    for (idx2, k) in city.roads.iter().enumerate() {
                        if idx == idx2 {
                            continue;
                        }
                        for dk in 0..k.points.len() - 1 {
                            let a = k.points[dk];
                            let b = k.points[dk + 1];
                            if a == p1 || b == p1 {
                                continue;
                            }
                            if distance_to_line_segmment(l, a, b) < 20 {
                                hit_count += 1;
                            }
                        }
                    }
                }
                if !hit_count < 10 {
                    spikes.push((p1, s1, s2));
                }
                let scale_2 = -((random::<u64>() % 16 + 8) as f32);
                let s3 = Point::from_vec2(p1.as_vec2() + delta * scale_2 / 2.0);
                let s4 = Point::from_vec2(p1.as_vec2() + delta * scale_2);
                let s0 = [p1, s3, s4];
                let mut hit_count = 0;
                for l in s0 {
                    for k in &city.roads {
                        for dk in 0..k.points.len() - 1 {
                            let a = k.points[dk];
                            let b = k.points[dk + 1];
                            if a == p1 || b == p1 {
                                continue;
                            }
                            if distance_to_line_segmment(l, a, b) < 20 {
                                hit_count += 1;
                            }
                        }
                    }
                }
                if hit_count < 10 {
                    spikes.push((p1, s3, s4));
                }
            }
        }
    }
    for i in spikes {
        let r = Road {
            points: vec![i.0, i.1, i.2],
        };
        city.roads.push(r);
    }
    'outer: loop {
        let mut should_remove = None;
        for (idx, i) in city.roads.iter().enumerate() {
            let mut offset = 0i64;
            for p in &i.points {
                let mut min_dist = 1000000i64;
                for (idx2, j) in city.roads.iter().enumerate() {
                    if idx == idx2 {
                        continue;
                    }
                    for k in 0..j.points.len() - 1 {
                        let dx = distance_to_line_segmment(*p, j.points[k], j.points[k + 1]) as i64;
                        if dx < min_dist {
                            min_dist = dx;
                        }
                    }
                }
                offset += min_dist;
            }
            if (offset as f64) * (i.points.len() as f64) < 100.0 {
                should_remove = Some(idx);
            }
        }
        if let Some(idx) = should_remove {
            city.roads.remove(idx);
            continue 'outer;
        }
        break;
    }
    //let hit_set = std::sync::Mutex::new(HashSet::new());
    for r in (0..500).rev() {
        let tmp_queue: Vec<(Point, f32)> = (0..628)
            .into_par_iter()
            .flat_map(|theta_0| {
                let r = r;
                let x = ((theta_0 as f32 / 100.).cos() * r as f32) as i32 + 500;
                let y = ((theta_0 as f32 / 100.).sin() * r as f32) as i32 + 500;
                let p = Point { x, y };
                /*let mut t0 = hit_set.lock().unwrap();
                if t0.contains(&p) {
                    return None;
                } else {
                    for dy in -4..=4 {
                        for dx in -4..=4 {
                            let tmp = Point {
                                x: p.x + dx,
                                y: p.y + dy,
                            };
                            //     t0.insert(tmp);
                        }
                    }
                    t0.insert(p);
                }
                drop((t0));*/
                let mut theta = 0.0;
                let mut closest_distance = 5000;
                for i in &city.roads {
                    for j in 1..i.points.len() {
                        let dist = distance_to_line_segmment(p, i.points[j], i.points[j - 1]);
                        if dist < closest_distance {
                            closest_distance = dist;
                            let delta =
                                (i.points[j - 1].as_vec2() - i.points[j].as_vec2()).normalized();
                            theta = delta.angle_to(Vector2::new(1.0, 0.0));
                        }
                        if dist < 4 {
                            return None;
                        }
                    }
                }
                if closest_distance < 12 {
                    Some((p, theta))
                } else {
                    None
                }
            })
            .collect();
        for i in tmp_queue {
            point_queue.push(i);
        }
    }
    println!("point queue length:{}", point_queue.len());
    point_queue
}
impl City {
    pub fn can_place_building(&self, building: &Building) -> bool {
        let vs = self
            .buildings
            .iter()
            .map(|i| i.bounds.check_collision(&building.bounds))
            .find(|x| *x)
            .is_some();
        if vs {
            return false;
        }
        let vs: bool = self
            .roads
            .iter()
            .map(|i| {
                for j in 0..i.points.len() - 1 {
                    let p0 = i.points[j];
                    let p1 = i.points[j + 1];
                    if building.bounds.distance_to_line(p0, p1) < 2 {
                        return false;
                    }
                }
                true
            })
            .find(|x| !*x)
            .is_some();
        !vs
    }

    pub fn draw(&self) {
        let mut img = Image::gen_image_color(1000, 1000, Color::WHITE);
        for i in &self.roads {
            for j in 0..i.points.len() - 1 {
                img.draw_line_v(
                    i.points[j].as_vec2(),
                    i.points[j + 1].as_vec2(),
                    Color::BLACK,
                );
            }
        }
        for k in &self.buildings {
            let points = k.bounds.vertices();
            img.draw_line_v(points[0], points[1], Color::BLACK);
            img.draw_line_v(points[0], points[2], Color::BLACK);
            img.draw_line_v(points[3], points[1], Color::BLACK);
            img.draw_line_v(points[3], points[2], Color::BLACK);
        }
        img.export_image("test.png");
        let mut img = Image::gen_image_color(1000, 1000, Color::WHITE);
        for k in &self.buildings {
            let points = k.bounds.vertices();
            img.draw_line_v(points[0], points[1], Color::BLACK);
            img.draw_line_v(points[0], points[2], Color::BLACK);
            img.draw_line_v(points[3], points[1], Color::BLACK);
            img.draw_line_v(points[3], points[2], Color::BLACK);
        }
        img.export_image("city.png");
    }
}
