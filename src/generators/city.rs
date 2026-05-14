use std::collections::{HashSet, VecDeque};

use rand::{random, random_bool};
use raylib::{color::Color, texture::Image};

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
    while let Some(_) = grow_city(&mut city, &mut point_queue, 2, 10) {
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
    queue: &mut Queue<(Point, f32)>,
    min_dist: i32,
    max_dist: i32,
) -> Option<()> {
    None
}

pub fn set_up_roads(city: &mut City) -> Queue<(Point, f32)> {
    fn random_point() -> Point {
        let x = (random::<u64>() % 1000) as i32;
        let y = (random::<u64>() % 1000) as i32;
        Point { x, y }
    }
    fn next_point(last: Point, second_to_last: Point) -> Point {
        let delta = (last.as_vec2() - second_to_last.as_vec2()).normalized();
        let distance = (random::<u64>() % 16 + 20) as f32;
        let dtheta = ((random::<u64>() % 64) as i32 - 32) as f32 / (128.);
        let new_point = last.as_vec2() + delta.rotated(dtheta) * distance;
        Point::from_vec2(new_point)
    }
    //if bool true push_back
    fn next_point_from_list(list: &VecDeque<Point>) -> (bool, Point) {
        if random_bool(0.5) {
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
    let mut point_queue: Queue<(Point, f32)> =
        Queue::new_cmp(|a: &(Point, f32), b: &(Point, f32)| {
            let cnt = Vector2::new(500., 500.);
            if b.0.as_vec2().distance_to(cnt) < a.0.as_vec2().distance_to(cnt) {
                std::cmp::Ordering::Less
            } else if b.0.as_vec2().distance_to(cnt) == a.0.as_vec2().distance_to(cnt) {
                std::cmp::Ordering::Equal
            } else {
                std::cmp::Ordering::Greater
            }
        });
    for _ in 0..=50 {
        let mut v = VecDeque::new();
        let p0 = random_point();
        let p1 = {
            let delta = Vector2::new(1.0, 0.0);
            let distance = (random::<u64>() % 16 + 20) as f32;
            let dtheta = ((random::<u64>() % 628) as i32) as f32 / (100.);
            let new_point = p0.as_vec2() + delta.rotated(dtheta) * distance;
            Point::from_vec2(new_point)
        };
        v.push_back(p0);
        v.push_back(p1);
        let mut last = p1;
        let mut dist = p1.as_vec2().distance_to(p0.as_vec2());
        let max_dist = (random::<u64>() % 200 + 200) as f32;
        while dist < max_dist {
            let (back, mut p) = next_point_from_list(&v);
            'it: for j in &city.roads {
                for k in &j.points {
                    if k.as_vec2().distance_to(p.as_vec2()) < 50. {
                        p = *k;
                        break 'it;
                    }
                }
            }
            if back {
                v.push_back(p);
            } else {
                v.push_front(p);
            }
            let d = last.as_vec2().distance_to(p.as_vec2());
            dist += d;
            last = p;
        }
        city.roads.push(Road { points: v.into() });
    }
    point_queue
}
impl City {
    pub fn can_place_building(&self, building: &Building) -> bool {
        for i in &self.roads {
            for j in 0..i.points.len() - 1 {
                let p0 = i.points[j];
                let p1 = i.points[j + 1];
                if building.bounds.distance_to_line(p0, p1) < 2 {
                    return false;
                }
            }
        }
        for i in &self.buildings {
            if i.bounds.check_collision(&building.bounds) {
                return false;
            }
        }
        true
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
    }
}
