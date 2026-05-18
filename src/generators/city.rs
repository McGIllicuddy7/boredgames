use core::f32;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    f32::consts::PI,
    process::id,
    sync::{Mutex, RwLock},
};

use rand::{random, random_bool, rng, seq::SliceRandom};
use raylib::{color::Color, texture::Image};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use serde::de;

use crate::builder::noise_1d_layered;

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
    let par = true;
    let start_road = Road {
        points: vec![Point { x: 450, y: 500 }, Point { x: 550, y: 500 }],
    };
    let mut city = City {
        buildings: Vec::new(),
        roads: Vec::new(),
    };
    city.roads.push(start_road);
    let mut point_queue = set_up_roads_radial(&mut city);
    println!("point count:{}", point_queue.len());
    if par {
        grow_city_par(&mut city, &mut point_queue, building_count);
    } else {
        while city.buildings.len() < building_count {
            if grow_city(&mut city, &mut point_queue, 0, 0).is_none() {
                break;
            }
            println!("{}", city.buildings.len());
        }
    }

    println!("cost count:{}", COST_COUNT.lock().unwrap());
    println!("building_count:{}", city.buildings.len());
    city
}

pub fn grow_city(
    city: &mut City,
    queue: &mut Vec<(Point, f32)>,
    _min_dist: i32,
    _max_dist: i32,
) -> Option<()> {
    while let Some((p, theta)) = queue.pop() {
        let max_h = 14 + (random::<u64>() % 4) as i32;
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

pub fn grow_city_par(city: &mut City, queue: &mut Vec<(Point, f32)>, count: usize) -> Option<()> {
    let queue = Mutex::new(queue);
    let c2 = RwLock::new(city);
    (0..std::thread::available_parallelism().unwrap().get())
        .into_par_iter()
        .for_each(|_| {
            loop {
                let point = queue.lock().unwrap().pop();
                let Some((p, theta)) = point else {
                    return;
                };
                let max_h = 12 + (random::<u64>() % 8) as i32;
                'tri: for height in (7..=max_h).rev() {
                    for width in (7..=max_h).rev() {
                        let rat = height as f32 / width as f32;
                        if rat < 0.8 || rat > 1.5 {
                            continue;
                        }
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
                        loop {
                            let city = c2.read().unwrap();
                            let mut ln = city.buildings.len();
                            if ln >= count {
                                break;
                            }
                            if city.can_place_building(&b) {
                                drop(city);
                                let mut city = c2.write().unwrap();
                                if city.buildings.len() == ln {
                                    city.buildings.push(b);
                                    let l = city.buildings.len();
                                    drop(city);
                                    println!("building count:{:#?}", l);
                                    break 'tri;
                                } else {
                                    ln = city.buildings.len();
                                }
                            } else {
                                continue 'tri;
                            }
                        }
                    }
                }
            }
        });
    None
}

static COST_COUNT: Mutex<usize> = Mutex::new(0);
fn cost(city: &City, par: bool) -> i64 {
    *COST_COUNT.lock().unwrap() += 1;
    //return random();
    let degeneracy = Mutex::new(0);
    let mut center = Vector2::zero();
    let mut pc = 0;
    for i in &city.roads {
        for j in &i.points {
            center += j.as_vec2();
            pc += 1;
        }
    }
    let point_count = pc;
    center /= (pc as f32);
    let neighbors_set: HashMap<(usize, usize), Vec<((usize, usize), i32)>> = city
        .roads
        .iter()
        .enumerate()
        .flat_map(|(idx, i)| {
            let mut set = HashMap::new();
            let mut points: Vec<_> = (0..i.points.len() - 1).collect();
            points.shuffle(&mut rng());
            for idx2 in points.into_iter().take(50) {
                let p = &i.points[idx2];
                let mut neighbors = Vec::new();
                let point = city.roads[idx].points[idx2];
                for (idx, i) in city.roads.iter().enumerate() {
                    let mut points: Vec<_> = (0..i.points.len() - 1).collect();
                    points.shuffle(&mut rng());
                    for j in points.into_iter().take(50) {
                        let p1 = i.points[j];
                        let p2 = i.points[j + 1];
                        let d = distance_to_line_segmment(point, p1, p2) as f32;
                        if d < 20. {
                            *degeneracy.lock().unwrap() += 1;
                        }
                        if *p == p1 {
                            if j > 0 {
                                let p2 = i.points[j - 1];
                                let ds = (p2.as_vec2().distance_to(p1.as_vec2()) + d) as i32;
                                neighbors.push(((idx, j), ds));
                            }
                            if j < i.points.len() - 1 {
                                let p2 = i.points[j + 1];
                                let ds = (p2.as_vec2().distance_to(p1.as_vec2()) + d) as i32;
                                neighbors.push(((idx, j + 1), ds));
                            }
                        }
                    }
                }
                set.insert((idx, idx2), neighbors);
            }
            set
        })
        .collect();
    let euc_dist = Mutex::new(0);
    let centroid_dist = Mutex::new(0);
    let calc = |v: (usize, &Road)| {
        let (idx, i) = v;
        for (idx2, _j) in i.points.iter().enumerate() {
            let td = _j.as_vec2().distance_to(center) as i64;
            let td2 = _j.as_vec2().distance_to(Vector2::new(500., 500.));
            if td2 > 500. {
                *centroid_dist.lock().unwrap() += 1000000;
            } else {
                *centroid_dist.lock().unwrap() += td.isqrt();
            }
            let distances = super::utils::distance_table(
                &(idx, idx2),
                city,
                |id: &(usize, usize), _city: &City| {
                    if let Some(v) = neighbors_set.get(id) {
                        return v.clone();
                    } else {
                        return Vec::new();
                    }
                },
            );
            let mut dist = 0;
            for (idx, i) in city.roads.iter().enumerate() {
                for k in 0..i.points.len() {
                    let dstar = 1000 as i64;
                    if k < i.points.len() - 1 {
                        let x0 = i.points[k];
                        let x1 = i.points[k + 1];
                        let d = distance_to_line_segmment(*_j, x0, x1);
                        if d < 10 && !(x0 == *_j || x1 == *_j) {
                            *euc_dist.lock().unwrap() -= 100;
                        } else {
                            *euc_dist.lock().unwrap() += (d + 10).isqrt() as i64;
                        }
                    }
                    if let Some(x) = distances.get(&(idx, k)) {
                        let d = i.points[k].as_vec2().distance_to(_j.as_vec2()) as i32;
                        if d < 20 {
                            *euc_dist.lock().unwrap() -= 100;
                        }
                        if (*x as i64) < dstar {
                            dist += *x as i64;
                        } else {
                            dist += dstar;
                        }
                    } else {
                        dist += dstar;
                    }
                }
            }
            _ = dist;
            //centroid_dist += dist;
        }
    };
    if par {
        city.roads.par_iter().enumerate().for_each(calc);
    } else {
        city.roads.iter().enumerate().for_each(calc);
    }
    let degeneracy = *degeneracy.lock().unwrap();
    let centroid_dist = *centroid_dist.lock().unwrap() / 100;
    let euc_dist = *euc_dist.lock().unwrap();
    let cost = (euc_dist - centroid_dist * 10) / point_count;
    cost - degeneracy
}

fn try_update_city_extend(city: &City, try_road_idx: usize, try_point_idx: usize) -> (City, i64) {
    let mut max_point = city.roads[try_road_idx].points[try_point_idx];
    let base = max_point;
    let mut max_dist = 0;
    let base_angle = if try_point_idx == 0 {
        let p1 = city.roads[try_road_idx].points[try_point_idx + 1];
        let p0 = city.roads[try_road_idx].points[try_point_idx];
        let theta = (p0.as_vec2() - p1.as_vec2())
            .normalized()
            .angle_to(Vector2::new(1.0, 0.0));
        theta
    } else if try_point_idx == city.roads[try_road_idx].points.len() - 1 {
        let p1 = city.roads[try_road_idx].points[try_point_idx];
        let p0 = city.roads[try_road_idx].points[try_point_idx - 1];
        let theta = (p1.as_vec2() - p0.as_vec2())
            .normalized()
            .angle_to(Vector2::new(1.0, 0.0));

        theta
    } else {
        let p1 = city.roads[try_road_idx].points[try_point_idx];
        let p0 = city.roads[try_road_idx].points[try_point_idx - 1];
        let theta1 = (p1.as_vec2() - p0.as_vec2())
            .normalized()
            .angle_to(Vector2::new(1.0, 0.0));
        let p2 = city.roads[try_road_idx].points[try_point_idx + 1];
        let p3 = city.roads[try_road_idx].points[try_point_idx];
        let theta2 = (p2.as_vec2() - p3.as_vec2())
            .normalized()
            .angle_to(Vector2::new(1.0, 0.0));
        (theta1 + theta2) / 2.
    };
    let mut angle = base_angle;
    let mut hit = false;
    for rad in 20..=25 {
        for theta in 0..16 {
            let rad = rad * 4 + 5;
            let theta = (theta) as f32 * PI / 8.
                + base_angle
                + ((random::<u64>() % 10) as i32 - 5) as f32 / 2.;
            let p0 = base.as_vec2()
                + Vector2::new((theta as f32).cos(), (theta as f32).sin()).normalized()
                    * (rad as f32);
            let mut min = 50000.0;
            for (idx, j) in city.roads.iter().enumerate() {
                for idx2 in 0..j.points.len() - 1 {
                    if idx2 == try_point_idx && idx == try_road_idx
                        || idx2 + 1 == try_point_idx && idx == try_road_idx
                    {
                        continue;
                    }
                    let d2 = distance_to_line_segmment(
                        Point::from_vec2(p0),
                        j.points[idx2],
                        j.points[idx2 + 1],
                    ) as f32;
                    if d2 < min {
                        min = d2;
                    }
                }
            }
            if (min as i32 + (random::<u64>() % 10) as i32 - 5) > max_dist {
                max_dist = min as i32;
                hit = true;
                max_point = Point::from_vec2(p0);
                angle = theta;
            }
        }
    }
    if !hit {
        return (city.clone(), cost(&city, false));
    }
    let mut city_2 = city.clone();
    if (angle - base_angle).abs() > 1000.0 {
        city_2.roads.push(Road {
            points: vec![base, max_point],
        })
    } else {
        if try_point_idx == 0 {
            city_2.roads[try_road_idx].points.insert(0, max_point);
        } else if try_point_idx == city_2.roads[try_road_idx].points.len() - 1 {
            city_2.roads[try_road_idx].points.push(max_point);
        } else {
            city_2.roads.push(Road {
                points: vec![base, max_point],
            })
        }
    }

    let cost = cost(&city_2, false);
    (city_2, cost)
}
pub fn set_up_roads(city: &mut City) -> Vec<(Point, f32)> {
    let mut c2 = city.clone();
    c2.roads = vec![
        Road {
            points: vec![Point { x: 480, y: 500 }, Point { x: 520, y: 500 }],
        },
        Road {
            points: vec![Point { x: 500, y: 480 }, Point { x: 500, y: 520 }],
        },
    ];
    let mut last_cost = -1000;
    let mut failed_count = 0;
    let mut dead_set: HashSet<(usize, usize)> = HashSet::new();
    for i in 0..250 {
        let mut set = Vec::new();
        for (idx, i) in c2.roads.iter().enumerate() {
            for j in 0..i.points.len() {
                if dead_set.contains(&(idx, j)) {
                    continue;
                }
                if j == 0 || j == i.points.len() - 1 || random_bool(0.5) {
                    set.push((idx, j));
                }
            }
        }
        set.shuffle(&mut rng());
        let mut cost_set = set
            .par_iter()
            .take(10)
            .map(|x| {
                let (idx, j) = *x;
                let mut c3 = try_update_city_extend(&c2, idx, j);
                c3.1 += (random::<u64>() % 100) as i64 - 50;
                (c3, *x)
            })
            .collect::<Vec<_>>();
        for (idx, i) in c2.roads.iter().enumerate() {
            if random_bool(0.8) && cost_set.len() > 3 {
                continue;
            }
            for (idx2, k) in c2.roads.iter().enumerate() {
                if dead_set.contains(&(idx, idx2)) {
                    continue;
                }
                if idx == idx2 {
                    continue;
                }
                if idx2 < idx {
                    continue;
                }
                let mut avg_dist = 0;
                let mut nearest_dist = i.points[0].as_vec2().distance_to(k.points[0].as_vec2());
                let mut nearest_pair = (i.points[0], k.points[0]);
                let mut intersects = false;
                for j in &i.points {
                    if idx == idx2 {
                        continue;
                    }
                    for l in &k.points {
                        let d = l.as_vec2().distance_to(j.as_vec2());
                        let d = d * d;
                        if d < nearest_dist {
                            nearest_pair = (*j, *l);
                            nearest_dist = d;
                        }
                        if d < 1. {
                            intersects = true;
                        }
                        avg_dist += d as i64;
                    }
                }
                if intersects {
                    continue;
                }
                let count = (i.points.len() + k.points.len()) as i64;
                avg_dist /= count;
                if avg_dist.isqrt() < 500 && avg_dist.isqrt() > 0 {
                    let r = Road {
                        points: vec![
                            nearest_pair.0,
                            Point::from_vec2(
                                (nearest_pair.0.as_vec2() + nearest_pair.1.as_vec2()) / 2.,
                            ),
                            nearest_pair.1,
                        ],
                    };
                    let mut tc = c2.clone();
                    tc.roads.push(r);
                    let cost = cost(&tc, true) + (random::<u64>() % 100) as i64 - 50;
                    cost_set.push(((tc, cost), (idx, idx2)));
                }
            }
        }
        let mut max = -1000000000000000;
        let mut max_idx = 0;
        for (idx, i) in cost_set.iter().enumerate() {
            if i.0.1 > max {
                max = i.0.1;
                max_idx = idx;
            }
        }
        let tmp = cost_set.remove(max_idx);
        let tcost = tmp.0.1;
        //last_cost = tcost;
        c2 = tmp.0.0;
        dead_set.insert(tmp.1);
        println!("max_index:{}, idx:{}, cost:{}", max_idx, i, tmp.0.1);
    }
    *city = c2;
    let idxs = (0..city.roads.len()).collect::<Vec<usize>>();
    generate_point_set(city, 1)
}

pub fn set_up_roads_radial(city: &mut City) -> Vec<(Point, f32)> {
    city.roads.clear();
    let mut set: Vec<Point> = Vec::new();
    let center = Vector2::new(500., 500.);
    for _ in 0..20 {
        let theta = (random::<u64>() % 628) as f32 / 100.0;
        let dir = Vector2::new(theta.cos(), theta.sin());
        let mut points = Vec::new();
        'lp: for i in -25..=-((random::<u64>() % 10) as i32) {
            let i = i * 25;
            let p0 = dir * (i as f32) + center;
            let offset_x =
                (noise_1d_layered(p0.x as i32, p0.y as i32, 0.1, "radial roads x", 2) - 0.5) * 500.;
            let offset_y =
                (noise_1d_layered(p0.x as i32, p0.y as i32, 0.1, "radial roads y", 2) - 0.5) * 500.;
            let p0 = p0 + Vector2::new(offset_x, offset_y);
            for j in &set {
                let p1 = j.as_vec2();
                if p1.distance_to(p0) < 25. {
                    points.push(*j);
                    continue 'lp;
                }
            }
            set.push(Point::from_vec2(p0));
            points.push(Point::from_vec2(p0));
        }
        city.roads.push(Road { points });
        let mut points = Vec::new();
        'lp: for i in ((random::<u64>() % 10) as i32)..=25 {
            let i = i * 25;
            let p0 = dir * (i as f32) + center;
            let offset_x =
                (noise_1d_layered(p0.x as i32, p0.y as i32, 0.1, "radial roads x", 2) - 0.5) * 500.;
            let offset_y =
                (noise_1d_layered(p0.x as i32, p0.y as i32, 0.1, "radial roads y", 2) - 0.5) * 500.;
            let p0 = p0 + Vector2::new(offset_x, offset_y);
            for j in &set {
                let p1 = j.as_vec2();
                if p1.distance_to(p0) < 25. {
                    points.push(*j);
                    continue 'lp;
                }
            }
            set.push(Point::from_vec2(p0));
            points.push(Point::from_vec2(p0));
        }
        city.roads.push(Road { points });
    }
    for r in 1..20 {
        let rad = r as f32 * 25.;
        let mut points = Vec::new();
        for theta in 0..20 as i32 {
            let theta = (theta as f32 * 6.28) / 20.;
            let pos = Vector2::new(theta.cos(), theta.sin()) * rad + center;
            let mut min_point = pos;
            let mut min_dist = 10000.;
            for i in &set {
                let d = i.as_vec2().distance_to(pos);
                if d < min_dist {
                    min_dist = d;
                    min_point = i.as_vec2();
                }
            }
            points.push(Point::from_vec2(min_point));
        }
        city.roads.push(Road { points });
    }
    for _ in 0..=40 {
        for i in 0..20 {
            let theta = ((random::<u64>() % 100) as f32) / 100.;
            let mut points = Vec::new();
            let base_rad = (random::<u64>() % 10) as i32 + i;
            for r in base_rad..=base_rad + (random::<u64>() % 10 + 2) as i32 {
                let rad = r as f32 * 25.;
                let theta = (theta as f32 * 6.28);
                let pos = Vector2::new(theta.cos(), theta.sin()) * rad + center;
                let mut min_point = pos;
                let mut min_dist = 10000.;
                for i in &set {
                    let d = i.as_vec2().distance_to(pos);
                    if d < min_dist {
                        min_dist = d;
                        min_point = i.as_vec2();
                    }
                }
                points.push(Point::from_vec2(min_point));
            }
            city.roads.push(Road { points });
        }
    }
    generate_point_set(city, 1)
}

pub fn generate_point_set(city: &City, divisor: i32) -> Vec<(Point, f32)> {
    let mut point_queue = Vec::new();
    for r in (0..500 / divisor).rev() {
        // println!("r:{}", r);
        let tmp_queue: Vec<(Point, f32)> = (0..314)
            .into_iter()
            .flat_map(|theta_0| {
                let theta_0 = (theta_0 * 2) as f32;
                let r = r as i32;
                let bx = (theta_0 / 100.).cos();
                let by = (theta_0 / 100.).sin();
                let x = (bx * r as f32) as i32 + 500;
                let y = (by * r as f32) as i32 + 500;
                let p = Point { x, y };
                let mut theta = 0.0;
                let mut closest_distance = 5000;
                for (_idx, i) in city.roads.iter().enumerate() {
                    for j in 1..i.points.len() {
                        let dist = distance_to_line_segmment(p, i.points[j], i.points[j - 1]);
                        if dist < closest_distance {
                            closest_distance = dist;
                            let delta =
                                (i.points[j - 1].as_vec2() - i.points[j].as_vec2()).normalized();
                            theta = delta.angle_to(Vector2::new(1.0, 0.0));
                        }
                        if dist < 1 {
                            /*println!(
                                "broke on:{} {}, point:{:#?}, start:{:#?}, end:{:#?}",
                                idx,
                                j,
                                p,
                                i.points[j],
                                i.points[j - 1],
                            );*/
                            return None;
                        }
                    }
                }
                //     println!("x:{}, y:{}, dist:{}", x, y, closest_distance);
                if closest_distance < 20 {
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
    //println!("point queue length:{}", point_queue.len());
    point_queue
}
impl City {
    pub fn can_place_building(&self, building: &Building) -> bool {
        let mut closest_building_dist = 10000000;
        let b_vertices = building.bounds.vertices();
        let vs = self
            .buildings
            .iter()
            .map(|i| {
                let i_vertices = i.bounds.vertices();
                let b_edges = [
                    (b_vertices[0], b_vertices[1]),
                    (b_vertices[0], b_vertices[2]),
                    (b_vertices[3], b_vertices[1]),
                    (b_vertices[3], b_vertices[2]),
                ];
                let i_edges = [
                    (i_vertices[0], i_vertices[1]),
                    (i_vertices[0], i_vertices[2]),
                    (i_vertices[3], i_vertices[1]),
                    (i_vertices[3], i_vertices[2]),
                ];
                for e1 in b_edges {
                    for e2 in i_edges {
                        let dist = distance_between_line_segments(
                            Point::from_vec2(e1.0),
                            Point::from_vec2(e1.1),
                            Point::from_vec2(e2.0),
                            Point::from_vec2(e2.1),
                        );
                        if dist < closest_building_dist {
                            closest_building_dist = dist;
                        }
                    }
                }
                i.bounds.check_collision(&building.bounds)
            })
            .find(|x| *x)
            .is_some();
        if vs {
            return false;
        }
        let mut min_dist = 10000;
        let mut max_dist = 0;

        let vs: bool = self
            .roads
            .iter()
            .map(|i| {
                for j in 0..i.points.len() - 1 {
                    let p0 = i.points[j];
                    let p1 = i.points[j + 1];
                    let d = building.bounds.distance_to_line(p0, p1);
                    if d > max_dist {
                        max_dist = d;
                    }
                    if d < 1 {
                        return false;
                    }
                    if d < min_dist {
                        min_dist = d;
                    }
                }
                true
            })
            .find(|x| !*x)
            .is_some();
        if min_dist > 5 && max_dist > 50 && closest_building_dist < 10 {
            return false;
        }
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
            let b2 = k.bounds;
            //  b2.bounds.x += 1;
            // b2.bounds.y += 1;
            //b2.bounds.height -= 2;
            //b2.bounds.width -= 2;
            let points = b2.vertices();
            img.draw_line_v(points[0], points[1], Color::BLACK);
            img.draw_line_v(points[0], points[2], Color::BLACK);
            img.draw_line_v(points[3], points[1], Color::BLACK);
            img.draw_line_v(points[3], points[2], Color::BLACK);
        }
        img.export_image("city.png");
    }
}
