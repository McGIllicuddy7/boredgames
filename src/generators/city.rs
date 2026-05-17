use core::f32;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    f32::consts::PI,
};

use rand::{random, random_bool, rng, seq::SliceRandom};
use raylib::{color::Color, texture::Image};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
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
        let max_h = 14 + (random::<u64>() % 4) as i32;
        for height in (10..=max_h).rev() {
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
    fn cost(city: &City) -> i64 {
        let c = 10; 
        let mut point_count = 0;
        let mut average_distance = 0;
        let mut degeneracy = 0;
        let neighbors_set: HashMap<(usize, usize), Vec<((usize, usize), i32)>> = city
            .roads
            .iter()
            .enumerate()
            .flat_map(|(idx, i)| {
                let mut set = HashMap::new();
                for (idx2, p) in i.points.iter().enumerate() {
                    let mut neighbors = Vec::new();
                    let point = city.roads[idx].points[idx2];
                    for (idx, i) in city.roads.iter().enumerate() {
                        for j in 0..i.points.len() - 1 {
                            let p1 = i.points[j];
                            let p2 = i.points[j + 1];
                            let d = distance_to_line_segmment(point, p1, p2) as f32;
                            if d < 20. && !(*p == p1) {
                                degeneracy += 1;
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
        let mut euc_dist = 0;
        for (idx, i) in city.roads.iter().enumerate() {
            for (idx2, _j) in i.points.iter().enumerate() {
                point_count += 1;
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
                        let dstar = 10000 as i64;
                        if k<i.points.len()-1{
                            let x0 = i.points[k];
                            let x1 = i.points[k+1];
                            let d = distance_to_line_segmment(*_j,x0,x1);
                            if d<20 && d != 0{
                                euc_dist -= 100; 
                            }else{
                                euc_dist += d.isqrt() as i64;
                            }

                        } if let Some(x) = distances.get(&(idx, k)) {
                            if (*x as i64) < dstar {
                                dist += *x as i64;
                            } else {
                                dist += dstar * dstar;
                            }
                        } else {
                            dist += dstar * dstar;
                        }
                    }
                }
                average_distance += dist.isqrt();
            }
        }
        average_distance /= point_count as i64 * point_count as i64;
        degeneracy /= (point_count*point_count);
        //euc_dist/=point_count;
        let cost = euc_dist/1000;
        cost-average_distance - degeneracy
    }

    fn try_update_city_extend(
        city: &City,
        try_road_idx: usize,
        try_point_idx: usize, 
    ) -> (City, i64) {
        let mut max_point = city.roads[try_road_idx].points[try_point_idx];
        let base = max_point;
        let mut max_dist = 0;
        let base_angle = if try_point_idx == 0 {
            let p1 = city.roads[try_road_idx].points[try_point_idx + 1];
            let p0 = city.roads[try_road_idx].points[try_point_idx];
            let theta = (p1.as_vec2() - p0.as_vec2())
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
        for rad in 20..=25{
            for theta in 0..4 {
                let rad = rad * 2 + 5;
                let mut theta =
                    (theta) as f32 * PI / 2. + base_angle + (random::<u64>() % 10 - 5) as f32 / 60.;
                if try_point_idx == city.roads.len() - 1 {
                    theta = base_angle + (random::<u64>() % 100 - 50) as f32 / 240.;
                } else if try_point_idx == 0 {
                    theta = -base_angle + (random::<u64>() % 100 - 50) as f32 / 240.;
                };
                let theta = theta;
                let p0 = base.as_vec2()
                    + Vector2::new((theta as f32).cos(), (theta as f32).sin()) * (rad as f32);
                let mut min = 50000.0;
                for (idx, j) in city.roads.iter().enumerate() {
                    if idx == try_road_idx {
                        continue;
                    }
                    for k in &j.points {
                        let d2 = k.as_vec2().distance_to(p0);
                        if d2 < min {
                            min = d2;
                        }
                    }
                }
                if (min as i32) > max_dist {
                    max_dist = min as i32;
                    max_point = Point::from_vec2(p0);
                }
            }
        }
        let mut city_2 = city.clone();
        if base_angle.abs() > std::f32::consts::PI / 6.
            || city_2.roads[try_road_idx].points.len() >16
        {
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
        let cost = cost(&city_2);
        (city_2, cost)
    }
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
    let mut dead_set:HashSet<(usize, usize)> = HashSet::new(); 
    for i in 0..150 { 
        let mut set = Vec::new();
        for (idx, i) in c2.roads.iter().enumerate() {
            for j in 0..i.points.len() {
                if dead_set.contains(&(idx, j)) {
                    continue;
                }
                if j == 0 || j == i.points.len() - 1 || random_bool(0.01) {
                    set.push((idx, j));
                }
            }
        }
        set.shuffle(&mut rng()); 
        let mut cost_set = set.par_iter().take(10).map(|x|{
            let (idx, j) = *x;
            let c3 = try_update_city_extend(&c2, idx,j);
            (c3, *x)
        }).collect::<Vec<_>>();
        for (idx, i) in c2.roads.iter().enumerate() {
         
            'lp: for (idx2, k) in c2.roads.iter().enumerate() {
                if random_bool(0.8) {
                    continue;
                }
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
                if avg_dist.isqrt() < 250 && avg_dist.isqrt() > 0 {
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
                    let cost = cost(&tc);
                    cost_set.push(((tc, cost), (idx, idx2)));
                    continue 'lp;
                }
            }
        }
        let mut max = 0;
        let mut max_idx = 0;
        let bcost = last_cost;
        for (idx, i) in cost_set.iter().enumerate() {
            if i.0.1 > max {
                max = i.0.1;
                max_idx = idx;
            }
        }
        let tmp = cost_set.remove(max_idx);
        let tcost = tmp.0.1;
     /* if tcost < bcost {
            failed_count += 1;
            if failed_count > 16 {
                break;
            }
            continue;
        }*/
        failed_count = 0; 
        last_cost = tcost;
        c2 = tmp.0.0; 
        //dead_set.insert(tmp.1);
        println!("max_index:{}, idx:{}, cost:{}", max_idx, i, tmp.0.1);
        if i % 3== 0 {
            c2.draw();
        }
    }
    *city = c2;
    let  idxs = (0..city.roads.len()).collect::<Vec<usize>>(); 
    generate_point_set(city, 1)
}

pub fn generate_point_set(city: &City, divisor: i32) -> Vec<(Point, f32)> {
    let mut point_queue = Vec::new();
    for r in (0..500 / divisor).rev() {
        let r = r * divisor;
        let tmp_queue: Vec<(Point, f32)> = (0..314)
            .into_iter()
            .flat_map(|theta_0| {
                let theta_0 = theta_0 * 2;
                let r = r + ((random::<u64>()) % (divisor as u64)) as i32;
                let x = ((theta_0 as f32 / 100.).cos() * r as f32) as i32 + 500;
                let y = ((theta_0 as f32 / 100.).sin() * r as f32) as i32 + 500;
                let p = Point { x, y };
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
        let vs = self
            .buildings
            .iter()
            .map(|i| i.bounds.check_collision(&building.bounds))
            .find(|x| *x)
            .is_some();
        if vs {
            return false;
        }
        let mut min_dist = 10000;
        let vs: bool = self
            .roads
            .iter()
            .map(|i| {
                for j in 0..i.points.len() - 1 {
                    let p0 = i.points[j];
                    let p1 = i.points[j + 1];
                    let d = building.bounds.distance_to_line(p0, p1);
                    if d < 4 {
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
        if min_dist > 8 {
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
