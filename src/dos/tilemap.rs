use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;

use super::SysHandle;

use super::common::*;
#[repr(C)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Sprite {
    //please don't write to this :3, thank you
    pub id: KeyType,
    pub x_pos: i16,
    pub y_pos: i16,
    pub width: u16,
    pub height: u16,
    pub image_id: KeyType,
    pub display_name: Arc<str>,
    pub layer: u8,
}

pub const BACKGROUND_LAYER: usize = 0;
pub const TILE_LAYER: usize = 1;
pub const HIDDEN_LAYER: usize = 2;
pub const FOREGROUND_LAYER: usize = 3;
pub const LAYER_COUNT: usize = 4;

pub type KeyType = u64;
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMapData {
    //in drawn order
    pub background: String,
    pub layers: [Arc<[AtomicU64]>; LAYER_COUNT],
    pub draw_table: BTreeMap<KeyType, String>,
    //in drawn order
    pub sprites: BTreeMap<KeyType, Sprite>,
    pub name_table: BTreeMap<KeyType, String>,
    pub map_width: i32,
    pub map_height: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMap {
    data: TileMapData,
    draw_width: i32,
    draw_height: i32,
    center_x: i32,
    center_y: i32,
    allow_mouse_input: bool,
    selected: Option<KeyType>,
}

impl TileMap {
    pub fn new(width: i32, height: i32, background: String, draw_table: Vec<String>) -> Self {
        let mut dc1 = Vec::new();
        for _ in 0..width * height {
            dc1.push(AtomicU64::new(0));
        }
        let mut dc2 = Vec::new();
        for _ in 0..width * height {
            dc2.push(AtomicU64::new(0));
        }
        let mut dc3 = Vec::new();
        for _ in 0..width * height {
            dc3.push(AtomicU64::new(0));
        }
        let mut dc4 = Vec::new();
        for _ in 0..width * height {
            dc4.push(AtomicU64::new(0));
        }

        let layers = [dc1.into(), dc2.into(), dc3.into(), dc4.into()];
        let mut table = BTreeMap::new();
        for (i, s) in draw_table.iter().enumerate() {
            let idx = (i + 1) as KeyType;
            table.insert(idx, s.clone());
        }
        Self {
            draw_width: width,
            draw_height: height,
            center_x: width / 2,
            center_y: height / 2,
            data: TileMapData {
                background,
                map_width: width,
                map_height: height,
                layers,
                draw_table: table,
                sprites: BTreeMap::new(),
                name_table: BTreeMap::new(),
            },
            allow_mouse_input: false,
            selected: None,
        }
    }

    pub fn get_tile(&self, layer: usize, x: i32, y: i32) -> KeyType {
        self.data.layers[layer][(y * self.data.map_width + x) as usize]
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn set_tile<T: Into<KeyType>>(&self, layer: usize, x: i32, y: i32, to: T) {
        self.data.layers[layer][(y * self.data.map_width + x) as usize]
            .store(to.into(), std::sync::atomic::Ordering::SeqCst);
    }

    pub fn create_sprite(
        &mut self,
        pos_x: i32,
        pos_y: i32,
        w: i32,
        h: i32,
        img: KeyType,
        layer: usize,
    ) -> KeyType {
        let mut id = 1;
        for i in 1..=KeyType::MAX {
            id = i;
            if !self.data.sprites.contains_key(&i) {
                break;
            }
        }
        if id == KeyType::MAX {
            0
        } else {
            let s = Sprite {
                id,
                x_pos: pos_x as i16,
                y_pos: pos_y as i16,
                width: w as u16,
                height: h as u16,
                image_id: img,
                display_name: String::new().into(),
                layer: layer as u8,
            };
            self.data.sprites.insert(id, s);
            id
        }
    }
    pub fn create_sprite_with_id(
        &mut self,
        id: KeyType,
        pos_x: i32,
        pos_y: i32,
        w: i32,
        h: i32,
        img: KeyType,
        layer: usize,
    ) {
        let s = Sprite {
            id,
            x_pos: pos_x as i16,
            y_pos: pos_y as i16,
            width: w as u16,
            height: h as u16,
            image_id: img,
            display_name: String::new().into(),
            layer: layer as u8,
        };
        self.data.sprites.insert(id, s);
    }
    pub fn get_sprite(&self, id: KeyType) -> Sprite {
        self.data.sprites[&id].clone()
    }

    pub fn set_sprite(&mut self, id: KeyType, sprite: Sprite) {
        *self.data.sprites.get_mut(&id).unwrap() = sprite;
    }
    pub fn get_sprites(&self) -> BTreeMap<KeyType, Sprite> {
        self.data.sprites.clone()
    }
    pub fn get_data(&self) -> TileMapData {
        self.data.clone()
    }
    pub fn set_sprites(&mut self, sprites: BTreeMap<KeyType, Sprite>) {
        self.data.sprites = sprites;
    }

    pub fn set_data(&mut self, data: TileMapData) {
        self.data = data;
    }

    pub fn delete_sprite(&mut self, id: KeyType) {
        self.data.sprites.remove(&id);
    }

    pub fn check_sprite_exists(&self, id: KeyType) -> bool {
        self.data.sprites.contains_key(&id)
    }

    pub fn get_image_id(&self, name: &str) -> Option<KeyType> {
        for i in 0..self.data.draw_table.len() {
            if let Some(x) = self.data.draw_table.get(&(i as KeyType)) {
                if x == name {
                    return Some(i as KeyType + 1);
                }
            } else {
                break;
            }
        }
        None
    }

    //no u cannot unload images, l+ratio
    pub fn load_image(&mut self, name: &str) -> KeyType {
        let mut idx = 1;
        for i in 1..KeyType::MAX {
            if !self.data.draw_table.contains_key(&i) {
                idx = i;
                break;
            }
        }
        self.data.draw_table.insert(idx, name.to_string());
        idx
    }
    pub fn set_draw_table(&mut self, names: &[String]) {
        self.data.draw_table.clear();
        for i in names {
            self.load_image(i);
        }
    }
    pub fn get_map_dimensions(&self) -> (i32, i32) {
        (self.data.map_width, self.data.map_height)
    }

    pub fn get_view_dimensions(&self) -> (i32, i32) {
        (self.draw_width, self.draw_height)
    }

    pub fn set_view_dimensions(&mut self, w: i32, h: i32) {
        self.draw_width = w;
        self.draw_height = h;
    }

    pub fn get_view_center(&self) -> Pos2 {
        Pos2 {
            x: self.center_x,
            y: self.center_y,
        }
    }

    pub fn set_view_center(&mut self, x: i32, y: i32) {
        self.center_x = x;
        self.center_y = y;
    }

    pub fn screen_to_map_coord(
        &mut self,
        x: i32,
        y: i32,
        base_x: i32,
        base_y: i32,
        w: i32,
        h: i32,
    ) -> (i16, i16) {
        let start_x = self.center_x - self.draw_width / 2;
        let start_y = self.center_y - self.draw_height / 2;
        let xshift = w as f64 / self.draw_width as f64;
        let yshift = h as f64 / self.draw_height as f64;
        (
            ((x - base_x - start_x) as f64 / xshift) as i16,
            ((y - base_y - start_y) as f64 / yshift) as i16,
        )
    }
    //if a sprite was clicked returns where it was globally and its id, if a tile was clicked returns its indexes.
    pub fn draw(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        handle: &mut SysHandle,
        draw_hidden: bool,
    ) -> (Option<Pos2>, Option<(KeyType, Pos2)>) {
        let mut queue = Vec::new();
        let start_x = self.center_x - self.draw_width / 2;
        let start_y = self.center_y - self.draw_height / 2;
        let xshift = w as f64 / self.draw_width as f64;
        let yshift = h as f64 / self.draw_height as f64;
        let mut dx;
        let mut dy;
        let pos = handle.get_absolute_pos(Pos2 { x, y });
        let mut hit = None;
        let mut out = None;
        queue.push(DrawCall::Rectangle {
            x: pos.x - 1,
            y: pos.y - 1,
            w: w + 2,
            h: h + 2,
            color: handle.get_theme().background_color,
            drop_shadow: handle.get_theme().shadows,
            outline: handle.get_theme().outline,
        });
        queue.push(DrawCall::DrawImage {
            x: pos.x,
            y: pos.y,
            h,
            w,
            contents_ident: self.data.background.clone(),
        });
        for i in 0..LAYER_COUNT {
            dx = x as f64;
            dy = y as f64;
            if i == HIDDEN_LAYER && !draw_hidden {
                continue;
            }
            for y in start_y..start_y + self.draw_height {
                for x in start_x..start_x + self.draw_width {
                    let tile = self.get_tile(i, x, y);
                    let xp = dx;
                    let yp = dy;
                    if handle.left_mouse_pressed() {
                        let rct = Rect {
                            x: xp as i32,
                            y: yp as i32,
                            w: xshift as i32,
                            h: yshift as i32,
                        };
                        if rct.check_collision(Pos2 {
                            x: handle.get_mouse_x(),
                            y: handle.get_mouse_y(),
                        }) {
                            hit = Some(Pos2 { x, y });
                        }
                    }
                    dx += xshift;
                    dy += yshift;
                    if tile == 0 {
                        continue;
                    }
                    let Some(name) = self.data.draw_table.get(&tile) else {
                        continue;
                    };
                    let dc = DrawCall::DrawImage {
                        x: xp as i32,
                        y: yp as i32,
                        h: yshift as i32,
                        w: xshift as i32,
                        contents_ident: name.clone(),
                    };
                    queue.push(dc);
                }
            }
        }
        for i in 0..LAYER_COUNT {
            if i == HIDDEN_LAYER && !draw_hidden {
                continue;
            }
            for (id, sprite) in &self.data.sprites {
                if sprite.layer as usize != i {
                    continue;
                }
                if self.selected.map(|i| i != *id).unwrap_or(false) {
                    continue;
                };
                let px = (sprite.x_pos as i32 - start_x) as f64 * xshift;
                let py = (sprite.y_pos as i32 - start_y) as f64 * yshift;
                let dx = px as i32 + x;
                let dy = py as i32 + y;
                let width = (sprite.width as f64 * xshift) as i32;
                let height = (sprite.height as f64 * yshift) as i32;
                let rect = Rect {
                    x: dx,
                    y: dy,
                    w: width,
                    h: height,
                };
                if rect.check_collision(Pos2 {
                    x: handle.get_mouse_x(),
                    y: handle.get_mouse_y(),
                }) && handle.left_mouse_pressed()
                {
                    out = Some((
                        *id,
                        Pos2 {
                            x: handle.get_mouse_x(),
                            y: handle.get_mouse_y(),
                        },
                    ));
                    if self.allow_mouse_input {
                        self.selected = Some(*id);
                    }
                }
                if sprite.image_id == 0 {
                    continue;
                }
                let name = self.data.draw_table.get(&sprite.image_id).unwrap();
                let dc = DrawCall::DrawImage {
                    x: dx,
                    y: dy,
                    h: height,
                    w: width,
                    contents_ident: name.clone(),
                };
                queue.push(dc);
            }
        }
        if self.allow_mouse_input {
            let (x, y) = self.screen_to_map_coord(
                handle.get_mouse_x(),
                handle.get_mouse_y(),
                pos.x,
                pos.y,
                w,
                h,
            );
            if handle.left_mouse_pressed() {
                for (id, sprite) in &self.data.sprites {
                    if sprite.x_pos == x && sprite.y_pos == y {
                        self.selected = Some(*id);
                    }
                }
            } else if handle.left_mouse_released() && self.selected.is_some() {
                let r = Rect {
                    x: pos.x,
                    y: pos.y,
                    w,
                    h,
                };
                if r.check_collision(Pos2 {
                    x: handle.get_mouse_x(),
                    y: handle.get_mouse_y(),
                }) {
                    let con = self.selected.unwrap();
                    let mut s = self.get_sprite(con);
                    s.x_pos = x;
                    s.y_pos = y;
                    self.set_sprite(con, s);
                }
                self.selected = None;
            }
            if self.selected.is_some() {
                let x = handle.get_mouse_x() - (xshift / 2.0) as i32;
                let y = handle.get_mouse_y() - (yshift / 2.0) as i32;
                let s = self.get_sprite(self.selected.unwrap());
                queue.push(DrawCall::DrawImage {
                    x,
                    y,
                    h: yshift as i32,
                    w: xshift as i32,
                    contents_ident: self.data.draw_table[&s.id].clone(),
                });
            }
        }
        handle.send_draw_calls(
            queue,
            Rect {
                x: pos.x,
                y: pos.y,
                w,
                h,
            },
        );
        (hit, out)
    }

    pub fn enable_mouse(&mut self) {
        self.allow_mouse_input = true;
        self.selected = None;
    }
    pub fn disable_mouse(&mut self) {
        self.allow_mouse_input = false;
        self.selected = None;
    }
}
