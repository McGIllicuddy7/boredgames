use raylib::camera::Camera3D;
use raylib::ffi::KeyboardKey;
pub use raylib::math::BoundingBox;
use raylib::math::{Quaternion, Ray, Vector4};
use raylib::models::{Mesh, Model, RaylibMesh, RaylibModel};
pub use raylib::prelude::{Color, Vector3};
use raylib::prelude::{RaylibDraw, RaylibDraw3D, RaylibMode3DExt, RaylibTextureModeExt};
use raylib::shaders::{RaylibShader, Shader};
use raylib::texture::RenderTexture2D;
use raylib::{RaylibHandle, RaylibThread};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use crate::dos::SysHandle;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GObject {
    pub model_name: Arc<str>,
    pub position: Vector3,
    pub rotation: Quaternion,
    pub bounds: BoundingBox,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct GLight {
    pub pos: Vector3,
    pub color: Vector4,
    pub radius: f32,
    pub bounds: Vector3,
    pub enabled: i32,
    pub distances: [f32; 16],
}

pub fn cardinals() -> [Vector3; 16] {
    [
        Vector3::new(0.0, 0.0, 1.0).normalized(),
        Vector3::new(0.0, 1.0, 0.0).normalized(),
        Vector3::new(0.0, 1.0, 1.0).normalized(),
        Vector3::new(1.0, 0.0, 0.0).normalized(),
        Vector3::new(1.0, 0.0, 1.0).normalized(),
        Vector3::new(1.0, 1.0, 0.0).normalized(),
        Vector3::new(1.0, 1.0, 1.0).normalized(),
        Vector3::new(0.0, 1.0, -1.0).normalized(),
        Vector3::new(1.0, 0.0, -1.0).normalized(),
        Vector3::new(1.0, -1.0, 0.0).normalized(),
        Vector3::new(0.0, -1.0, 1.0).normalized(),
        Vector3::new(-1.0, 0.0, 1.0).normalized(),
        Vector3::new(-1.0, 1.0, 0.0).normalized(),
        Vector3::new(0.0, -1.0, -1.0).normalized(),
        Vector3::new(-1.0, 0.0, -1.0).normalized(),
        Vector3::new(-1.0, -1.0, 0.0).normalized(),
    ]
}
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct GLightId {
    id: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct GObjectId {
    id: u32,
}

impl GObjectId {
    pub fn invalid() -> Self {
        Self { id: 0 }
    }
    pub fn get(&self) -> u32 {
        self.id
    }
    pub fn is_valid(&self) -> bool {
        self.id != 0
    }
}
impl GLightId {
    pub fn invalid() -> Self {
        Self { id: 0 }
    }
    pub fn get(&self) -> u32 {
        self.id
    }
    pub fn is_valid(&self) -> bool {
        self.id != 0
    }
}
impl GLight {
    pub fn new(pos: Vector3, color: Color, radius: f32) -> Self {
        Self {
            pos,
            color: Vector4 {
                x: color.r as f32 / 255.,
                y: color.g as f32 / 255.,
                z: color.b as f32 / 255.,
                w: color.a as f32 / 255.,
            },
            radius,
            bounds: Vector3 {
                x: radius * 2.,
                y: radius * 2.,
                z: radius * 2.,
            },
            enabled: 1,
            distances: [radius; 16],
        }
    }
    pub fn empty() -> Self {
        Self {
            pos: Vector3::zero(),
            color: Vector4::new(0., 0., 0., 0.),
            radius: 0.0,
            bounds: Vector3 {
                x: 0.,
                y: 0.,
                z: 0.,
            },
            enabled: 0,
            distances: [0.0; 16],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Scene {
    pub cam_pos: Vector3,
    pub cam_rot: Quaternion,
    pub objects: BTreeMap<GObjectId, GObject>,
    pub lights: BTreeMap<GLightId, GLight>,
    pub f_debug_lights: bool,
}
impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            cam_pos: Vector3::zero(),
            cam_rot: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            objects: BTreeMap::new(),
            lights: BTreeMap::new(),
            f_debug_lights: false,
        }
    }

    pub fn create_object(&mut self, object: GObject) -> GObjectId {
        for i in 1..u32::MAX {
            let id = GObjectId { id: i };
            if let std::collections::btree_map::Entry::Vacant(e) = self.objects.entry(id) {
                e.insert(object);
                return id;
            }
        }
        GObjectId { id: 0 }
    }

    pub fn destroy_object(&mut self, id: GObjectId) {
        self.objects.remove(&id);
    }

    pub fn get_object(&self, id: GObjectId) -> Option<&GObject> {
        self.objects.get(&id)
    }

    pub fn set_object(&mut self, id: GObjectId, object: GObject) -> Option<GObject> {
        if !self.objects.contains_key(&id) {
            return Some(object);
        }
        self.objects.insert(id, object);
        None
    }

    pub fn get_object_mut(&mut self, id: GObjectId) -> Option<&mut GObject> {
        self.objects.get_mut(&id)
    }
    pub fn get_object_clone(&mut self, id: GObjectId) -> Option<GObject> {
        self.objects.get_mut(&id).map(|i| i.clone())
    }

    pub fn create_light(&mut self, light: GLight) -> GLightId {
        for i in 1..u32::MAX {
            let id = GLightId { id: i };
            if let std::collections::btree_map::Entry::Vacant(e) = self.lights.entry(id) {
                e.insert(light);
                return id;
            }
        }
        GLightId { id: 0 }
    }

    pub fn destroy_light(&mut self, id: GObjectId) {
        self.objects.remove(&id);
    }

    pub fn get_light(&self, id: GLightId) -> Option<&GLight> {
        self.lights.get(&id)
    }

    pub fn get_light_clone(&self, id: GLightId) -> Option<GLight> {
        self.lights.get(&id).cloned()
    }

    pub fn set_light(&mut self, id: GLightId, light: GLight) -> Option<GLight> {
        if !self.lights.contains_key(&id) {
            return Some(light);
        }
        self.lights.insert(id, light);
        None
    }

    pub fn get_light_mut(&mut self, id: GLightId) -> Option<&mut GLight> {
        self.lights.get_mut(&id)
    }

    pub fn camera_input(&mut self, handle: &SysHandle) {
        if handle.is_key_down(KeyboardKey::KEY_Q) {
            self.cam_rot = Quaternion::from_euler(0.0, -0.01, 0.0) * self.cam_rot;
        }
        if handle.is_key_down(KeyboardKey::KEY_E) {
            self.cam_rot = Quaternion::from_euler(0.0, 0.01, 0.0) * self.cam_rot;
        }
        if handle.is_key_down(KeyboardKey::KEY_R) {
            self.cam_rot = Quaternion::from_euler(0.01, 0.00, 0.0) * self.cam_rot;
        }
        if handle.is_key_down(KeyboardKey::KEY_F) {
            self.cam_rot = Quaternion::from_euler(-0.01, 0.0, 0.0) * self.cam_rot;
        }
        let forward = Vector3::forward().transform_with(self.cam_rot.to_matrix());
        let right = Vector3::right().transform_with(self.cam_rot.to_matrix());
        if handle.is_key_down(KeyboardKey::KEY_W) {
            self.cam_pos += forward / 30.0;
        }
        if handle.is_key_down(KeyboardKey::KEY_S) {
            self.cam_pos -= forward / 30.0;
        }
        if handle.is_key_down(KeyboardKey::KEY_D) {
            self.cam_pos -= right / 30.0;
        }
        if handle.is_key_down(KeyboardKey::KEY_A) {
            self.cam_pos += right / 30.0;
        }
    }
}

pub struct SceneRenderer {
    pub loaded_meshes: HashMap<Arc<str>, Model>,
    pub to_load: HashSet<Arc<str>>,
    pub shader: Option<Shader>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub should_draw: bool,
}
impl Default for SceneRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneRenderer {
    pub fn new() -> Self {
        Self {
            loaded_meshes: HashMap::new(),
            to_load: HashSet::new(),
            shader: None,
            x: 0,
            y: 0,
            w: 1200,
            h: 900,
            should_draw: false,
        }
    }
    pub fn setup_render(
        &mut self,
        scene: &Scene,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
    ) {
        let directions = cardinals();
        if self.shader.is_none() {
            self.shader = Some(handle.load_shader(
                thread,
                Some("shaders/lighting.vs"),
                Some("shaders/lighting.fs"),
            ));
            let mut msh = handle
                .load_model_from_mesh(thread, unsafe {
                    Mesh::gen_mesh_cube(thread, 1.0, 1.0, 1.0).make_weak()
                })
                .unwrap();
            unsafe { (*msh.materials).shader = **self.shader.as_ref().unwrap() };
            self.loaded_meshes.insert("box".into(), msh);
        }
        let shade = self.shader.as_mut().unwrap();
        let pos_loc = shade.get_shader_location("lightPositions");
        let col_loc = shade.get_shader_location("lightColors");
        let rad_loc = shade.get_shader_location("lightRadii");
        let enable_loc = shade.get_shader_location("lightEnabled");
        let dist_lock = shade.get_shader_location("lightDistances");
        let dir_lock = shade.get_shader_location("directions");
        let lights: Vec<GLight> = scene.lights.iter().map(|i| *i.1).collect();
        let _objects: Vec<GObject> = scene.objects.values().map(|i| i.clone()).collect();
        let mut nearest_set: Vec<GLight> = take_min(
            &mut |i: &GLight| (i.pos - scene.cam_pos).length() as f64,
            &lights,
            16,
        );

        for i in &mut nearest_set {
            let pos = i.pos;
            for (idx, j) in directions.iter().enumerate() {
                let ray_dir = *j;
                let mut min_dist = i.radius;
                for (_, obj) in &scene.objects {
                    let col = RotBox::new(obj.bounds, obj.rotation, obj.position);
                    if let Some(dist) = col.ray_cast(pos, ray_dir) {
                        if dist < min_dist {
                            min_dist = dist;
                        }
                    }
                }
                i.distances[idx] = min_dist;
            }
        }
        let mut positions = [Vector3::zero(); 16];
        let mut colors = [Vector4::new(0.0, 0.0, 0.0, 0.0); 16];
        let mut radii = [0.0; 16];
        let mut enabled = [0; 16];
        let mut distances = [0.0; 256];
        for i in nearest_set.iter().enumerate() {
            positions[i.0] = i.1.pos;
            colors[i.0] = i.1.color;
            radii[i.0] = i.1.radius;
            enabled[i.0] = i.1.enabled;
            for j in 0..16 {
                distances[i.0 * 16 + j] = i.1.distances[j];
            }
        }
        shade.set_shader_value_v(pos_loc, &positions);
        shade.set_shader_value_v(col_loc, &colors);
        shade.set_shader_value_v(rad_loc, &radii);
        shade.set_shader_value_v(enable_loc, &enabled);
        shade.set_shader_value_v(dist_lock, &distances);
        shade.set_shader_value_v(dir_lock, &directions);
    }

    pub fn draw_scene(
        &mut self,
        scene: &Scene,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
    ) {
        let mut draw = handle.begin_texture_mode(thread, target);
        let cam = Camera3D::perspective(
            scene.cam_pos,
            Vector3::forward().transform_with(scene.cam_rot.to_matrix()) + scene.cam_pos,
            Vector3::up().transform_with(scene.cam_rot.to_matrix()),
            90.0,
        );
        draw.clear_background(Color::BLACK);
        let mut draw = draw.begin_mode3D(cam);
        for (_, i) in scene.objects.iter() {
            if !self.loaded_meshes.contains_key(&i.model_name) {
                self.to_load.insert(i.model_name.clone());
                continue;
            }
            let md = self.loaded_meshes.get_mut(&i.model_name).unwrap();
            md.transform = i.rotation.to_matrix().into();
            draw.draw_model(&md, i.position, 1.0, Color::WHITE);
        }
        if scene.f_debug_lights {
            let directions = cardinals();
            for (_, i) in scene.lights.iter() {
                for (idx, j) in i.distances.iter().enumerate() {
                    draw.draw_line_3D(i.pos, i.pos + directions[idx] * *j, Color::RED);
                }
            }
        }
    }

    #[allow(unused)]
    pub fn render(
        &mut self,
        scene: &Scene,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
    ) {
        self.setup_render(scene, handle, thread);
        self.draw_scene(scene, handle, thread, target);
    }
}

pub fn take_min<T: Clone>(
    get_value: &mut impl FnMut(&T) -> f64,
    slice: &[T],
    count: usize,
) -> Vec<T> {
    let mut out = slice.to_vec();
    out.sort_by(|x, y| {
        let vx = get_value(x);
        let vy = get_value(y);
        if vx > vy {
            std::cmp::Ordering::Greater
        } else if vx == vy {
            std::cmp::Ordering::Equal
        } else {
            std::cmp::Ordering::Less
        }
    });
    let cot = if slice.len() < count {
        slice.len()
    } else {
        count
    };
    out.drain(0..cot).collect()
}

pub struct RotBox {
    pub bounds: BoundingBox,
    pub rotation: Quaternion,
    pub position: Vector3,
}
pub fn box_points(bx: BoundingBox) -> [Vector3; 8] {
    [
        Vector3::new(bx.min.x, bx.min.y, bx.min.z),
        Vector3::new(bx.min.x, bx.min.y, bx.max.z),
        Vector3::new(bx.min.x, bx.max.y, bx.min.z),
        Vector3::new(bx.min.x, bx.max.y, bx.max.z),
        Vector3::new(bx.max.x, bx.min.y, bx.min.z),
        Vector3::new(bx.max.x, bx.min.y, bx.max.z),
        Vector3::new(bx.max.x, bx.max.y, bx.min.z),
        Vector3::new(bx.max.x, bx.max.y, bx.max.z),
    ]
}
pub fn bound_points(points: [Vector3; 8]) -> BoundingBox {
    let mut min = points[0];
    let mut max = points[0];
    for i in points {
        if i.x < min.x {
            min.x = i.x;
        }
        if i.y < min.y {
            min.y = i.y;
        }
        if i.z < min.z {
            min.z = i.z;
        }
        if i.x > max.x {
            max.x = i.x;
        }
        if i.y > max.y {
            max.y = i.y;
        }
        if i.z > max.z {
            max.z = i.z;
        }
    }
    BoundingBox::new(min, max)
}
impl RotBox {
    pub fn new(bounds: BoundingBox, rotation: Quaternion, position: Vector3) -> Self {
        Self {
            bounds,
            rotation,
            position,
        }
    }

    pub fn as_points(&self) -> [Vector3; 8] {
        let mut ps = box_points(self.bounds);
        for i in &mut ps {
            i.rotate_by(self.rotation);
        }
        for i in &mut ps {
            *i += self.position;
        }
        ps
    }
    pub const fn const_normal_vectors() -> [Vector3; 6] {
        [
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(0.0, -1.0, 0.0),
            Vector3::new(-1.0, 0.0, 0.0),
        ]
    }

    pub fn sap_vectors(&self, other: &Self) -> [Vector3; 15] {
        let base_normals = [
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
        ];
        let svectors = [
            Vector3::forward().rotate_by(self.rotation),
            Vector3::left().rotate_by(self.rotation),
            Vector3::up().rotate_by(self.rotation),
        ];
        let ovectors = [
            Vector3::forward().rotate_by(other.rotation),
            Vector3::left().rotate_by(other.rotation),
            Vector3::up().rotate_by(other.rotation),
        ];

        [
            base_normals[0].rotate_by(self.rotation),
            base_normals[1].rotate_by(self.rotation),
            base_normals[2].rotate_by(self.rotation),
            base_normals[0].rotate_by(other.rotation),
            base_normals[1].rotate_by(other.rotation),
            base_normals[2].rotate_by(other.rotation),
            svectors[0].cross(ovectors[0]),
            svectors[0].cross(ovectors[1]),
            svectors[0].cross(ovectors[2]),
            svectors[1].cross(ovectors[0]),
            svectors[1].cross(ovectors[1]),
            svectors[1].cross(ovectors[2]),
            svectors[2].cross(ovectors[0]),
            svectors[2].cross(ovectors[1]),
            svectors[2].cross(ovectors[2]),
        ]
    }

    pub fn distance(&self, other: &Self) -> f32 {
        let spoints = self.as_points();
        let opoints = other.as_points();
        let vecs = self.sap_vectors(other);
        let mut min_depth = std::f32::INFINITY;
        for i in vecs {
            let mut smax = spoints[0].dot(i);
            let mut smin = spoints[0].dot(i);
            for j in spoints {
                let dot = j.dot(i);
                if dot > smax {
                    smax = dot;
                }
                if dot < smin {
                    smin = dot;
                }
            }
            let mut omax = opoints[0].dot(i);
            let mut omin = opoints[0].dot(i);
            for j in opoints {
                let dot = j.dot(i);
                if dot > omax {
                    omax = dot;
                }
                if dot < omin {
                    omin = dot;
                }
            }
            let d = if smin > omin && smin < omax {
                intersection(smin, smax, omin, omax)
            } else if smax > omin && smax < omax {
                intersection(smin, smax, omin, omax)
            } else if omin > smin && omin < omax {
                intersection(smin, smax, omin, omax)
            } else if omax >= smin && omax <= smax {
                intersection(smin, smax, omin, omax)
            } else if omin >= smax {
                omin - smax
            } else if smin >= omin {
                smin - omin
            } else {
                todo!()
            };
            if d < min_depth {
                min_depth = d;
            }
        }
        min_depth
    }

    pub fn check_collision(&self, other: &Self) -> bool {
        let spoints = self.as_points();
        let opoints = other.as_points();
        let vecs = self.sap_vectors(other);
        for i in vecs {
            let mut smax = spoints[0].dot(i);
            let mut smin = spoints[0].dot(i);
            for j in spoints {
                let dot = j.dot(i);
                if dot > smax {
                    smax = dot;
                }
                if dot < smin {
                    smin = dot;
                }
            }
            let mut omax = opoints[0].dot(i);
            let mut omin = opoints[0].dot(i);
            for j in opoints {
                let dot = j.dot(i);
                if dot > omax {
                    omax = dot;
                }
                if dot < omin {
                    omin = dot;
                }
            }
            if smin > omin && smin < omax {
            } else if smax > omin && smax < omax {
            } else if omin > smin && omin < omax {
            } else if omax >= smin && omax <= smax {
            } else if omin >= smax {
                return false;
            } else if smin >= omin {
                return false;
            } else {
                todo!()
            };
        }
        true
    }

    pub fn check_collision_normal(&self, other: &Self) -> Option<Vector3> {
        let spoints = self.as_points();
        let opoints = other.as_points();
        let vecs = self.sap_vectors(other);
        let mut min_depth = std::f32::INFINITY;
        let mut out = Vector3::zero();
        for i in vecs {
            let mut smax = spoints[0].dot(i);
            let mut smin = spoints[0].dot(i);
            for j in spoints {
                let dot = j.dot(i);
                if dot > smax {
                    smax = dot;
                }
                if dot < smin {
                    smin = dot;
                }
            }
            let mut omax = opoints[0].dot(i);
            let mut omin = opoints[0].dot(i);
            for j in opoints {
                let dot = j.dot(i);
                if dot > omax {
                    omax = dot;
                }
                if dot < omin {
                    omin = dot;
                }
            }
            let d = if smin > omin && smin < omax {
                intersection(smin, smax, omin, omax)
            } else if smax > omin && smax < omax {
                intersection(smin, smax, omin, omax)
            } else if omin > smin && omin < omax {
                intersection(smin, smax, omin, omax)
            } else if omax >= smin && omax <= smax {
                intersection(smin, smax, omin, omax)
            } else if omin >= smax {
                return None;
            } else if smin >= omin {
                return None;
            } else {
                todo!()
            };
            if d < min_depth {
                min_depth = d;
                out = i;
            }
        }
        Some(out)
    }

    pub fn ray_cast(&self, start: Vector3, direction: Vector3) -> Option<f32> {
        let start = (start - self.position).rotate_by(self.rotation.inverted());
        let col = self.bounds.get_ray_collision_box(Ray::new(
            start,
            direction.rotate_by(self.rotation.inverted()),
        ));
        if col.hit { Some(col.distance) } else { None }
    }
}

pub fn fmin(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

pub fn fmax(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

pub fn intersection(amin: f32, amax: f32, bmin: f32, bmax: f32) -> f32 {
    let damin = fmin((amin - bmin).abs(), (amin - bmax).abs());
    let damax = fmin((amax - bmin).abs(), (amax - bmax).abs());
    fmax(damax, damin)
}
