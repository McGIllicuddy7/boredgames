use rand::{random, seq::SliceRandom};
use raylib::{color::Color, math::Rectangle, texture::Image};
use serde::de::value;

use crate::libgui::{Bounds, Point, Widget};
pub const UP: usize = 0;
pub const DOWN: usize = 1;
pub const LEFT: usize = 2;
pub const RIGHT: usize = 3;
#[derive(Clone, Debug, Copy)]
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
                    x: min_x as f32 * 10. - 10.,
                    y: min_y as f32 * 10. - 10.,
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
}

#[derive(Clone, Debug)]
pub struct Room {
    pub points: Vec<Point>,
    pub boundary_positions: Vec<Point>,
}

pub fn generate_ground_floor(width: i32, height: i32) -> Floor {
    let mut floor = Floor::new_blank(width, height);
    let previous_floor = Floor::new_occupied(width, height);
    let mut has_another_room = false;
    let mut collection = Vec::new();
    while let Some(x) = generate_room(&mut floor, &previous_floor, has_another_room) {
        collection.push(x);
        has_another_room = true;
    }
    floor.rooms = collection;
    floor
}

pub fn post_process_floor(floor: &mut Floor, width: i32, height: i32, circular: bool) {
    'outer: loop {
        let cx = floor.width() / 2;
        let cy = floor.height() / 2;

        for i in 0..floor.rooms.len() {
            let mut hit = false;
            for j in &floor.rooms[i].points {
                let dx = j.x - cx;
                let dy = j.y - cy;
                if circular {
                    let r2 = (width + height) * (width + height) / 4;
                    if (dx * dx + dy * dy) > r2 {
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
                    floor.get_mut(j.x, j.y).sides = [SideKind::Empty; 4];
                }
                continue 'outer;
            };
        }
        break 'outer;
    }
    'outer: loop {
        let mut updated = false;
        for i in 0..floor.height() {
            for j in 0..floor.width() {
                if floor.get(j, i).is_occupied {
                    continue;
                }
                let mut count = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        if let Some(t) = floor.get_checked(j + dx, i + dy) {
                            if t.is_occupied && !t.is_wall {
                                count += 2;
                            } else if t.is_occupied {
                                count += 1;
                            }
                        }
                    }
                    if count >= 4 {
                        floor.get_mut(j, i).is_occupied = true;
                        floor.get_mut(j, i).is_wall = true;
                        updated = true;
                    }
                }
            }
        }
        println!("{}", updated);
        if !updated {
            break 'outer;
        }
    }
}

pub fn generate_room(floor: &mut Floor, previous: &Floor, has_another_room: bool) -> Option<Room> {
    let mut points: Vec<Point> = (0..floor.height())
        .flat_map(|y| (0..floor.width()).map(move |x| Point { x, y }))
        .collect();
    let mut r = Vec::new();
    let mut fs = 0;
    while !points.is_empty() {
        if fs > 10 {
            break;
        }
        let _idx = (random::<u64>() as usize) % points.len();
        let v = points.remove(_idx);
        if let Some(room) = try_generate_room(v, floor, previous, has_another_room) {
            fs += 1;
            r.push(room);
        }
    }
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
    return Some(v);
}

pub fn try_generate_room(
    point: Point,
    floor: &mut Floor,
    previous: &Floor,
    has_another_room: bool,
) -> Option<Room> {
    let mut values = Vec::new();
    let mut first = true;
    'gt: loop {
        let mut bases: Vec<(i32, i32)> = if first {
            (5..11).flat_map(|h| (5..11).map(move |w| (w, h))).collect()
        } else {
            (3..8).flat_map(|h| (3..8).map(move |w| (w, h))).collect()
        };
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
            if ratio > 2. && ratio < 3. {
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
                points,
                boundary_positions: border_positions,
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
