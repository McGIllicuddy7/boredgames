use std::{collections::HashSet, os::macos::raw::stat};

use libc::rand;
use rand::{random, seq::SliceRandom};
use raylib::{color::Color, math::Rectangle, texture::Image};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::de::{self, value};

use crate::libgui::{Bounds, Point, Widget};
pub const UP: usize = 0;
pub const DOWN: usize = 1;
pub const LEFT: usize = 2;
pub const RIGHT: usize = 3;
pub const DIRECTIONS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (0, 1)];
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum SideKind {
    Wall,
    Door,
    Window,
    Empty,
}
#[derive(Clone, Debug)]
pub struct Tile {
    pub sides: [SideKind; 4],
    pub is_occupied: bool,
    pub is_wall: bool,
}

#[derive(Clone, Debug)]
pub struct Floor {
    pub tiles: Vec<Tile>,
    pub width: i32,
    pub height: i32,
    pub rooms: Vec<Room>,
}

impl Floor {
    pub fn new_blank(width: i32, height: i32) -> Self {
        Self {
            tiles: vec![
                Tile {
                    sides: [SideKind::Empty; 4],
                    is_occupied: false,
                    is_wall: false,
                };
                (width * height) as usize
            ],
            width,
            height,
            rooms: Vec::new(),
        }
    }
    pub fn new_occupied(width: i32, height: i32) -> Self {
        Self {
            tiles: vec![
                Tile {
                    sides: [SideKind::Empty; 4],
                    is_occupied: true,
                    is_wall: true,
                };
                (width * height) as usize
            ],
            width,
            height,
            rooms: Vec::new(),
        }
    }
    pub fn get(&self, x: i32, y: i32) -> Tile {
        if x >= 0 && x < self.width && y < self.height && y >= 0 {
            return self.tiles[(y * self.width + x) as usize].clone();
        } else {
            todo!()
        }
    }

    pub fn get_checked(&self, x: i32, y: i32) -> Option<Tile> {
        if x >= 0 && x < self.width && y < self.height && y >= 0 {
            Some(self.tiles[(y * self.width + x) as usize].clone())
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> &mut Tile {
        if x >= 0 && x < self.width && y < self.height && y >= 0 {
            &mut self.tiles[(y * self.width + x) as usize]
        } else {
            todo!()
        }
    }

    pub fn get_mut_checked(&mut self, x: i32, y: i32) -> Option<&mut Tile> {
        if x >= 0 && x < self.width && y < self.height && y >= 0 {
            Some(&mut self.tiles[(y * self.width + x) as usize])
        } else {
            None
        }
    }

    pub fn set(&mut self, x: i32, y: i32, value: Tile) {
        *self.get_mut(x, y) = value
    }

    pub fn set_checked(&mut self, x: i32, y: i32, value: Tile) -> Result<(), Tile> {
        if let Some(v) = self.get_mut_checked(x, y) {
            *v = value;
            Ok(())
        } else {
            return Err(value);
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn render_debug(&self, name: &str) {
        let mut out = Image::gen_image_color(self.width() * 10, self.height() * 10, Color::BLACK);
        for i in &self.rooms {
            let mut min_x = 10000;
            let mut min_y = 10000;
            let mut max_x = 0;
            let mut max_y = 0;
            for j in &i.points {
                if j.x < min_x {
                    min_x = j.x;
                }
                if j.y < min_y {
                    min_y = j.y;
                }
                if j.x > max_x {
                    max_x = j.x;
                }
                if j.y > max_y {
                    max_y = j.y;
                }
                if self.get(j.x, j.y).is_wall {
                    out.draw_rectangle(j.x * 10, j.y * 10, 10, 10, Color::RED);
                }
            }
            out.draw_rectangle_lines(
                Rectangle {
                    x: min_x as f32 * 10.,
                    y: min_y as f32 * 10.,
                    width: (max_x - min_x + 1) as f32 * 10.,
                    height: (max_y - min_y + 1) as f32 * 10.,
                },
                1,
                Color::RED,
            );
        }
        for i in 0..self.height() {
            for j in 0..self.width() {
                if self.get(j, i).is_wall {
                    out.draw_rectangle(j * 10, i * 10, 10, 10, Color::RED);
                }
            }
        }
        out.export_image(name);
    }

    pub fn render(&self, name: &str) {
        let mut out = Image::gen_image_color(self.width() * 10, self.height() * 10, Color::BLACK);
        for y in 0..self.height() {
            for x in 0..self.width() {
                let p = self.get(x, y);
                if p.is_wall {
                    out.draw_rectangle(x * 10, y * 10, 10, 10, Color::BLACK);
                } else if p.is_occupied {
                    out.draw_rectangle(x * 10, y * 10, 10, 10, Color::WHITE);
                } else {
                    out.draw_rectangle(x * 10, y * 10, 10, 10, Color::GRAY);
                }
            }
        }
        for y in 0..self.height() {
            for x in 0..self.width() {
                let p = self.get(x, y);
                let points = [
                    (
                        Point {
                            x: x * 10,
                            y: y * 10,
                        },
                        Point {
                            x: (x + 1) * 10,
                            y: y * 10,
                        },
                    ),
                    (
                        Point {
                            x: x * 10,
                            y: (y + 1) * 10,
                        },
                        Point {
                            x: (x + 1) * 10,
                            y: (y + 1) * 10,
                        },
                    ),
                    (
                        Point {
                            x: x * 10,
                            y: y * 10,
                        },
                        Point {
                            x: x * 10,
                            y: (y + 1) * 10,
                        },
                    ),
                    (
                        Point {
                            x: (x + 1) * 10,
                            y: y * 10,
                        },
                        Point {
                            x: (x + 1) * 10,
                            y: (y + 1) * 10,
                        },
                    ),
                ];
                for i in 0..4 {
                    if p.sides[i] == SideKind::Wall {
                        let s = points[i].0;
                        let e = points[i].1;
                        out.draw_line(s.x, s.y, e.x, e.y, Color::BLACK);
                    }
                }
            }
        }
        out.export_image(name);
    }
}

#[derive(Clone, Debug)]
pub struct Room {
    pub points: Vec<Point>,
    pub boundary_positions: Vec<Point>,
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub connections: Vec<usize>,
}

pub fn generate_ground_floor(width: i32, height: i32) -> Floor {
    let mut floor = Floor::new_blank(width, height);
    let previous_floor = Floor::new_occupied(width, height);
    let mut has_another_room = false;
    while let Some(x) = generate_room(&mut floor, &previous_floor, has_another_room) {
        floor.rooms.push(x);
        has_another_room = true;
    }
    floor
}

pub fn post_process_floor(floor: &mut Floor, width: i32, height: i32, circular: bool) {
    for j in 0..floor.rooms.len() {
        let i = floor.rooms[j].clone();
        let mut border_positions = Vec::new();
        let point = Point {
            x: i.min_x,
            y: i.min_y,
        };
        let height = (i.max_y - i.min_y).abs();
        let width = (i.max_x - i.min_x).abs();
        let x_pos = i.min_x - 1;
        for dy in point.y..point.y + height {
            if let Some(t) = floor.get_checked(x_pos, dy) {
                if t.is_occupied {
                    border_positions.push(Point { x: x_pos, y: dy })
                }
            }
        }
        let x_pos = point.x + width + 1;
        for dy in point.y..point.y + height {
            if let Some(t) = floor.get_checked(x_pos, dy) {
                if t.is_occupied {
                    border_positions.push(Point { x: x_pos, y: dy })
                }
            }
        }
        let y_pos = point.y - 1;
        for dx in point.x..point.x + width {
            if let Some(t) = floor.get_checked(dx, y_pos) {
                if t.is_occupied {
                    border_positions.push(Point { x: dx, y: y_pos })
                }
            }
        }
        let y_pos = point.y + height + 1;
        for dx in point.x..point.x + width {
            if let Some(t) = floor.get_checked(dx, y_pos) {
                if t.is_occupied {
                    border_positions.push(Point { x: dx, y: y_pos })
                }
            }
        }
        floor.rooms[j].boundary_positions = border_positions;
    }
    let mut last_hit = false;
    for _ in 0..1 {
        'outer: loop {
            let cx = floor.width() / 2;
            let cy = floor.height() / 2;
            for i in 0..floor.rooms.len() {
                let mut hit = false;
                if floor.rooms[i].boundary_positions.len() < 5 {
                    hit = true;
                }
                for j in &floor.rooms[i].points {
                    let dx = j.x - cx;
                    let dy = j.y - cy;
                    if circular {
                        let r2 = ((width + height) * (width + height) / 4).isqrt();
                        if (dx * dx + dy * dy).isqrt() > r2 {
                            hit = true;
                            break;
                        }
                    } else {
                        if dx.abs() > width || dy.abs() > height {
                            hit = true;
                            break;
                        }
                    }
                }
                if hit {
                    let t = floor.rooms.remove(i);
                    for j in &t.points {
                        floor.get_mut(j.x, j.y).is_occupied = false;
                        floor.get_mut(j.x, j.y).is_wall = false;
                        floor.get_mut(j.x, j.y).sides = [SideKind::Empty; 4];
                        for k in &mut floor.rooms {
                            k.boundary_positions.retain(|i| *i != *j);
                        }
                    }
                    last_hit = true;
                    continue 'outer;
                };
            }
            if last_hit {
                last_hit = false;
                continue 'outer;
            }
            break 'outer;
        }
    }
    'outer: loop {
        let mut updated = false;
        for i in 0..floor.height() {
            for j in 0..floor.width() {
                if floor.get(j, i).is_occupied {
                    continue;
                }
                let mut count = 0;
                let mut acount = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        if let Some(t) = floor.get_checked(j + dx, i + dy) {
                            if t.is_occupied && !t.is_wall {
                                acount += 1;
                            } else if t.is_occupied {
                                count += 1;
                            }
                        }
                    }
                    if count >= 5 || acount >= 2 {
                        floor.get_mut(j, i).is_occupied = true;
                        floor.get_mut(j, i).is_wall = true;
                        updated = true;
                    }
                }
            }
        }
        if !updated {
            break 'outer;
        }
    }
    let mut fs = 0;
    'outer: loop {
        fs += 1;
        if fs > 10 {
            break;
        }
        let mut updated = false;
        for i in 0..floor.height() {
            for j in 0..floor.width() {
                if !floor.get(j, i).is_occupied || !floor.get(j, i).is_wall {
                    continue;
                }
                let mut count = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        if let Some(t) = floor.get_checked(j + dx, i + dy) {
                            if !t.is_occupied {
                                count += 1;
                            }
                        }
                    }
                    if count > 0 {
                        floor.get_mut(j, i).is_occupied = false;
                        floor.get_mut(j, i).is_wall = false;
                        updated = true;
                    }
                }
            }
        }
        if !updated {
            break 'outer;
        }
    }
    if floor.rooms.len() == 0 {
        return;
    }
    connect_rooms(floor);
}

pub fn generate_room(floor: &mut Floor, previous: &Floor, has_another_room: bool) -> Option<Room> {
    let mut points = if has_another_room {
        let mut x = HashSet::new();
        let mut r = Vec::new();
        for j in &floor.rooms {
            for k in j.get_boundary_points() {
                for dy in -5..=5 {
                    for dx in -5..=5 {
                        let p = Point {
                            x: k.x + dx,
                            y: k.y + dy,
                        };
                        if let Some(t) = floor.get_checked(p.x, p.y) {
                            if !t.is_occupied {
                                x.insert(p);
                            }
                        }
                    }
                }
            }
        }
        r.reserve(x.len());
        for i in x {
            r.push(i);
        }
        r
    } else {
        (0..floor.height())
            .flat_map(|y| (0..floor.width()).map(move |x| Point { x, y }))
            .collect()
    };

    points.shuffle(&mut rand::rng());
    let l = points.len().isqrt() * 4;
    let mut r: Vec<Room> = (0..l)
        .into_par_iter()
        .flat_map(|i| {
            for i in i..i + 1 {
                let v = *points.get(i)?;
                if let Some(room) = try_generate_room(v, floor, previous, has_another_room) {
                    return Some(room);
                }
            }
            return None;
        })
        .collect();
    if r.is_empty() {
        return None;
    }
    let mut idx = 0;
    let mut max_count = 0;
    for i in 0..r.len() {
        if r[i].boundary_positions.len() > max_count {
            idx = i;
            max_count = r[i].boundary_positions.len();
        }
    }
    let v = r.remove(idx);
    for i in &v.points {
        floor.get_mut(i.x, i.y).is_occupied = true;
    }
    for i in v.min_x..=v.max_x {
        floor.get_mut(i, v.min_y).sides[UP] = SideKind::Wall;
    }
    for i in v.min_x..=v.max_x {
        floor.get_mut(i, v.max_y).sides[DOWN] = SideKind::Wall;
    }
    for i in v.min_y..=v.max_y {
        floor.get_mut(v.min_x, i).sides[LEFT] = SideKind::Wall;
    }
    for i in v.min_y..=v.max_y {
        floor.get_mut(v.max_x, i).sides[RIGHT] = SideKind::Wall;
    }
    Some(v)
}

pub fn try_generate_room(
    point: Point,
    floor: &Floor,
    previous: &Floor,
    has_another_room: bool,
) -> Option<Room> {
    let mut lc = 0;
    for i in &floor.rooms {
        if (i.max_x - i.min_x).abs() >= 15 || (i.max_y - i.min_y).abs() >= 15 {
            let mut dist = 100000000;
            for dx in 0..30 {
                for y in i.min_y..=i.max_y {
                    for x in i.min_x..=i.max_x {
                        let d = ((point.x + dx - x) * (point.x + dx - x)
                            + (point.y - y) * (point.y - y));
                        if d < dist {
                            dist = d;
                        }
                    }
                }
            }
            for dy in 0..30 {
                for y in i.min_y..=i.max_y {
                    for x in i.min_x..=i.max_x {
                        let d = ((point.x - x) * (point.x - x)
                            + (point.y + dy - y) * (point.y + dy - y));
                        if d < dist {
                            dist = d;
                        }
                    }
                }
            }
            if dist < 256 {
                lc += 1;
            }
        }
    }
    let mut values = Vec::new();
    let mut first = true;
    'gt: loop {
        let mut bases: Vec<(i32, i32)> = if first {
            (5..10).flat_map(|h| (5..10).map(move |w| (w, h))).collect()
        } else {
            (3..8).flat_map(|h| (3..8).map(move |w| (w, h))).collect()
        };
        bases.reserve(120);
        if lc < 1 {
            for k in 2..=2 as i32 {
                for j in 10..=30 {
                    bases.push((k, j));
                    bases.push((j, k));
                }
            }
        }
        bases.shuffle(&mut rand::rng());
        let bases = bases;
        'outer: for (width, height) in bases {
            let ratio = {
                if width > height {
                    width as f32 / height as f32
                } else {
                    height as f32 / width as f32
                }
            };
            if ratio > 1.5 && ratio < 5. {
                continue;
            }
            let mut points = Vec::new();
            for dy in point.y..point.y + height {
                for dx in point.x..point.x + width {
                    if let Some(tmp) = previous.get_checked(dx, dy) {
                        if !tmp.is_occupied {
                            continue 'outer;
                        }
                    } else {
                        continue 'outer;
                    }
                    if let Some(tmp) = floor.get_checked(dx, dy) {
                        if tmp.is_occupied {
                            continue 'outer;
                        }
                    } else {
                        continue 'outer;
                    }
                    points.push(Point { x: dx, y: dy });
                }
            }
            let points = points;
            let mut hit_other_count = 0;
            let mut border_positions = Vec::new();
            let x_pos = point.x - 1;
            for dy in point.y..point.y + height {
                if let Some(t) = floor.get_checked(x_pos, dy) {
                    if t.is_occupied {
                        hit_other_count += 1;
                        border_positions.push(Point { x: x_pos, y: dy })
                    }
                }
            }
            let x_pos = point.x + width;
            for dy in point.y..point.y + height {
                if let Some(t) = floor.get_checked(x_pos, dy) {
                    if t.is_occupied {
                        hit_other_count += 1;
                        border_positions.push(Point { x: x_pos, y: dy })
                    }
                }
            }
            let y_pos = point.y - 1;
            for dx in point.x..point.x + width {
                if let Some(t) = floor.get_checked(dx, y_pos) {
                    if t.is_occupied {
                        hit_other_count += 1;
                        border_positions.push(Point { x: dx, y: y_pos })
                    }
                }
            }
            let y_pos = point.y + height;
            for dx in point.x..point.x + width {
                if let Some(t) = floor.get_checked(dx, y_pos) {
                    if t.is_occupied {
                        hit_other_count += 1;
                        border_positions.push(Point { x: dx, y: y_pos })
                    }
                }
            }
            if (hit_other_count < 5) && has_another_room {
                continue 'outer;
            }
            values.push(Room {
                min_y: point.y,
                min_x: point.x,
                max_x: point.x + width - 1,
                max_y: point.y + height - 1,
                points,
                boundary_positions: border_positions,
                connections: Vec::new(),
            });
        }
        if values.len() != 0 {
            break 'gt;
        } else if first {
            first = false;
        } else {
            break 'gt;
        }
    }
    if values.is_empty() {
        return None;
    }
    let mut max_count = 0;
    let mut max_idx = 0;
    for i in 0..values.len() {
        let offset = (random::<u64>() as usize) % 4;
        if values[i].points.len() + offset > max_count {
            max_count = values[i].boundary_positions.len() + offset;
            max_idx = i;
        }
    }
    let v = values.remove(max_idx);
    Some(v)
}

pub fn connect_rooms_old(floor: &mut Floor) {
    let old = floor.clone();
    let mut connected_set = HashSet::new();
    let mut base_list: Vec<usize> = (0..floor.rooms.len()).collect();
    let start = base_list.remove((random::<u64>() as usize) % base_list.len());
    connected_set.insert(start);
    let mut no_progress_count = 0;
    let mut red_count = vec![0; floor.rooms.len()];
    let mut best = old.clone();
    let mut best_count = 0;
    let mut best_connections = HashSet::new();
    'outer: loop {
        if no_progress_count > 2 {
            no_progress_count = 0;
            if base_list.is_empty() {
                *floor = best;
                println!("no valid base");
                connected_set = best_connections.clone();
                break 'outer;
            }
            for j in &mut red_count {
                *j = 0;
            }
            if connected_set.len() > best_count {
                best_count = connected_set.len();
                best_connections = connected_set.clone();
                best = floor.clone();
            }
            *floor = old.clone();
            let start = base_list.remove((random::<u64>() as usize) % base_list.len());
            connected_set.clear();
            connected_set.insert(start);
        }
        let mut progressed = false;
        let mut v: Vec<usize> = (0..floor.rooms.len()).collect();
        v.shuffle(&mut rand::rng());
        for j in v {
            if connected_set.contains(&j) {
                let mc = if (floor.rooms[j].max_x - floor.rooms[j].min_x).abs() > 10
                    || (floor.rooms[j].max_y - floor.rooms[j].min_y).abs() > 10
                {
                    20
                } else {
                    10
                };
                if red_count[j] > mc {
                    continue;
                }
                red_count[j] += 1;
            }
            let mut points = Vec::new();
            let v = &floor.rooms[j];
            let delt = if no_progress_count > 0 { -2 } else { 1 };
            for i in v.min_x + delt..=v.max_x - delt {
                points.push(Point {
                    x: i,
                    y: v.min_y - 1,
                });
            }
            for i in v.min_x + delt..=v.max_x - delt {
                points.push(Point {
                    x: i,
                    y: v.max_y + 1,
                });
            }
            for i in v.min_y + delt..=v.max_y - delt {
                points.push(Point {
                    x: v.min_x - 1,
                    y: i,
                });
            }
            for i in v.min_y + delt..=v.max_y - delt {
                points.push(Point {
                    x: v.max_x + 1,
                    y: i,
                });
            }
            points.shuffle(&mut rand::rng());
            'k: for k in 0..floor.rooms.len() {
                if connected_set.contains(&k) {
                    let mc = if (floor.rooms[k].max_x - floor.rooms[k].min_x).abs() > 10
                        || (floor.rooms[k].max_y - floor.rooms[k].min_y).abs() > 10
                    {
                        20
                    } else {
                        10
                    };
                    let mc1 = if (floor.rooms[j].max_x - floor.rooms[j].min_x).abs() > 10
                        || (floor.rooms[j].max_y - floor.rooms[j].min_y).abs() > 10
                    {
                        20
                    } else {
                        10
                    };
                    if red_count[k] > mc && red_count[j] > mc1 {
                        continue;
                    }
                    red_count[k] += 1;
                    let mut mutual = None;
                    for i in &points {
                        if floor.rooms[k].points.contains(i) {
                            mutual = Some(*i);
                            break;
                        }
                    }
                    if let Some(m) = mutual {
                        for i in 0..4 {
                            let p2 = Point {
                                x: m.x + DIRECTIONS[i].0,
                                y: m.y + DIRECTIONS[i].1,
                            };
                            if floor.rooms[j].points.contains(&p2) {
                                floor.get_mut(m.x, m.y).sides[i] = SideKind::Door;
                                let s2 = if i == UP {
                                    DOWN
                                } else if i == DOWN {
                                    UP
                                } else if i == LEFT {
                                    RIGHT
                                } else {
                                    LEFT
                                };
                                floor.get_mut(m.x, m.y).sides[i] = SideKind::Door;
                                floor.get_mut(p2.x, p2.y).sides[s2] = SideKind::Door;
                                connected_set.insert(j);
                                progressed = true;
                                no_progress_count = 0;
                                break 'k;
                            }
                        }
                    }
                }
            }
        }
        if connected_set.len() == floor.rooms.len() {
            break;
        }
        if !progressed {
            no_progress_count += 1;
        }
    }
    println!(
        "connected len:{} rooms len:{}, base:{start}",
        connected_set.len(),
        floor.rooms.len()
    );
    let mut r2 = Vec::new();
    for i in 0..floor.rooms.len() {
        if connected_set.contains(&i) {
            r2.push(floor.rooms[i].clone());
        } else {
            let x = floor.rooms[i].clone();
            for j in &x.points {
                floor.get_mut(j.x, j.y).is_occupied = false;
                floor.get_mut(j.x, j.y).is_wall = false;
                floor.get_mut(j.x, j.y).sides = [SideKind::Empty; 4];
                for k in &mut floor.rooms {
                    k.boundary_positions.retain(|i| *i != *j);
                }
            }
        }
    }
    floor.rooms = r2;
}

pub fn connect_rooms(floor: &mut Floor) {
    fn flood(floor: &Floor, point: Point, reachable_set: &mut HashSet<Point>) {
        reachable_set.insert(point);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if dx != 0 && dy != 0 {
                    continue;
                }
                let p = Point {
                    x: point.x + dx,
                    y: point.y + dy,
                };
                if reachable_set.contains(&p) {
                    continue;
                }
                if let Some(g) = floor.get_checked(p.x, p.y) {
                    if g.is_occupied && !g.is_wall {
                        flood(floor, p, reachable_set);
                    }
                }
            }
        }
    }
    let mut max_set = 0;
    if floor.rooms.len() == 0 {
        return;
    }
    let mut max_reachable = HashSet::new();
    for i in 0..floor.rooms.len() {
        let p0 = floor.rooms[i].points[0];
        let mut reachable_from = HashSet::new();
        flood(&floor, p0, &mut reachable_from);
        if reachable_from.len() > max_reachable.len() {
            max_reachable = reachable_from;
            max_set = i;
        }
    }
    let mut r2 = Vec::new();
    for i in 0..floor.rooms.len() {
        if max_reachable.contains(&floor.rooms[i].points[0]) {
            r2.push(floor.rooms[i].clone());
        }
    }
    let mut reachable_set = HashSet::new();
    reachable_set.insert(max_set);
    let mut stack = Vec::new();
    stack.push(max_set);
    let mut checked = HashSet::new();
    while let Some(i) = stack.pop() {
        if checked.contains(&i) {
            continue;
        }
        checked.insert(i);
        let mut bounds = floor.rooms[i].get_boundary_points_nice(0);
        bounds.shuffle(&mut rand::rng());
        let bounds = bounds;
        for j in 0..floor.rooms.len() {
            stack.push(j);
            if floor.rooms[i].connections.contains(&j) || floor.rooms[j].connections.contains(&i) {
                continue;
            }
            if reachable_set.contains(&j)
                && reachable_set.contains(&j)
                && (random::<u32>() % 4 == 0)
            {
                continue;
            }
            if i == j {
                continue;
            }
            let mut hit = None;
            for dj in &bounds {
                if floor.rooms[j].points.contains(dj) {
                    println!("{:#?}", dj);
                    hit = Some(*dj);
                }
            }
            if let Some(m) = hit {
                for k in 0..4 {
                    let p2 = Point {
                        x: m.x + DIRECTIONS[k].0,
                        y: m.y + DIRECTIONS[k].1,
                    };
                    if floor.rooms[i].points.contains(&p2) {
                        floor.get_mut(m.x, m.y).sides[k] = SideKind::Door;
                        let s2 = if k == UP {
                            DOWN
                        } else if k == DOWN {
                            UP
                        } else if k == LEFT {
                            RIGHT
                        } else {
                            LEFT
                        };
                        floor.get_mut(m.x, m.y).sides[k] = SideKind::Door;
                        floor.get_mut(p2.x, p2.y).sides[s2] = SideKind::Door;
                        floor.rooms[i].connections.push(j);
                        floor.rooms[j].connections.push(i);
                        reachable_set.insert(j);
                        stack.push(j);
                    }
                }
            }
        }
    }

    println!(
        "connected set:{}, room count:{}",
        reachable_set.len(),
        floor.rooms.len()
    );
}

impl Room {
    pub fn get_boundary_points(&self) -> Vec<Point> {
        let mut out = Vec::new();
        for i in &self.points {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    if dx != 0 && dy != 0 {
                        continue;
                    }
                    let p2 = Point {
                        x: i.x + dx,
                        y: i.y + dy,
                    };
                    if self.points.contains(&p2) {
                        continue;
                    }
                    if !out.contains(&p2) {
                        out.push(p2);
                    }
                }
            }
        }
        out
    }

    pub fn get_boundary_points_nice(&self, no_progress_count: i32) -> Vec<Point> {
        let mut points = Vec::new();
        let v = self;
        let delt = if no_progress_count > 0 { -2 } else { 1 };
        for i in v.min_x + delt..=v.max_x - delt {
            points.push(Point {
                x: i,
                y: v.min_y - 1,
            });
        }
        for i in v.min_x + delt..=v.max_x - delt {
            points.push(Point {
                x: i,
                y: v.max_y + 1,
            });
        }
        for i in v.min_y + delt..=v.max_y - delt {
            points.push(Point {
                x: v.min_x - 1,
                y: i,
            });
        }
        for i in v.min_y + delt..=v.max_y - delt {
            points.push(Point {
                x: v.max_x + 1,
                y: i,
            });
        }
        points
    }
}
