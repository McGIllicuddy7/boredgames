use raylib::camera::Camera3D;
use raylib::ffi::KeyboardKey;
use raylib::math::{Matrix, Quaternion, Vector4};
use raylib::models::{Model, RaylibMesh, RaylibModel};
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GLight {
    pub pos: Vector3,
    pub color: Color,
    pub direction: Vector3,
    pub up: Vector3,
    pub fov: f32,
    pub casts_shadows: bool,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Scene {
    pub cam_pos: Vector3,
    pub cam_rot: Quaternion,
    pub objects: BTreeMap<GObjectId, GObject>,
    pub lights: BTreeMap<GLightId, GLight>,
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
    pub shadow_map_textures: Vec<RenderTexture2D>,
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
            shadow_map_textures: Vec::new(),
            to_load: HashSet::new(),
            shader: None,
            x: 0,
            y: 0,
            w: 1200,
            h: 900,
            should_draw: false,
        }
    }
    pub fn render_scene(
        &mut self,
        scene: &Scene,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
        projections: &[Matrix],
        target: &mut RenderTexture2D,
    ) {
        let mut handle = handle.begin_texture_mode(thread, target);
        handle.clear_background(Color::BLACK);
        let shade = self.shader.as_mut().unwrap();
        let mat_locks = [
            shade.get_shader_location("lightVP0"),
            shade.get_shader_location("lightVP1"),
            shade.get_shader_location("lightVP2"),
            shade.get_shader_location("lightVP3"),
        ];
        let count_lock = shade.get_shader_location("light_count");
        let col_lock = shade.get_shader_location("ambient");
        let map_locks = [
            shade.get_shader_location("smap0"),
            shade.get_shader_location("smap1"),
            shade.get_shader_location("smap2"),
            shade.get_shader_location("smap3"),
        ];
        let dir_locks = shade.get_shader_location("lightDir");
        let light_col_locks = shade.get_shader_location("lightColor");
        let pos_locks = shade.get_shader_location("viewPos");
        let res_loc = shade.get_shader_location("shadowMapResolution");
        let cam_loc = shade.get_shader_location("cam_pos");
        shade.set_shader_value(cam_loc, scene.cam_pos);
        let fov_loc = shade.get_shader_location("fovs");
        shade.set_shader_value(res_loc, 1024);
        let mut dirs = [Vector3::zero(); 10];
        let mut cols = [Vector4::new(0.0, 0.0, 0.0, 0.0); 10];
        let mut poses = [Vector3::zero(); 10];
        let mut fovs = [0.0; 10];
        for (i, l) in scene.lights.iter().enumerate() {
            if i >= 10 {
                break;
            }
            dirs[i] = l.1.direction;
            cols[i].x = l.1.color.r as f32 / 255.;
            cols[i].y = l.1.color.g as f32 / 255.;
            cols[i].z = l.1.color.b as f32 / 255.;
            cols[i].w = l.1.color.a as f32 / 255.;
            poses[i] = l.1.pos;
            fovs[i] = l.1.fov;
        }
        shade.set_shader_value_v(light_col_locks, &cols);
        shade.set_shader_value_v(dir_locks, &dirs);
        shade.set_shader_value_v(pos_locks, &poses);
        shade.set_shader_value_v(fov_loc, &fovs);
        let cam = Camera3D::perspective(
            scene.cam_pos,
            Vector3::forward().transform_with(scene.cam_rot.to_matrix()) + scene.cam_pos,
            Vector3::up().transform_with(scene.cam_rot.to_matrix()),
            90.0,
        );
        shade.set_shader_value(count_lock, scene.lights.len() as i32);
        shade.set_shader_value(col_lock, Vector4::new(1.0, 1.0, 1.0, 1.0));
        let depth_lock = shade.get_shader_location("depth");
        shade.set_shader_value(depth_lock, 0);
        let mut draw = handle.begin_mode3D(cam);
        unsafe {
            raylib::ffi::rlSetClipPlanes(0.01, 1000.0);
        }
        for i in 0..projections.len() {
            shade.set_shader_value_matrix(mat_locks[i], projections[i]);
        }
        for i in 0..projections.len() {
            //  shade.set_shader_value_texture(map_locks[i], &self.shadow_map_textures[i]);
            unsafe {
                raylib::ffi::rlActiveTextureSlot(10 + i as i32);
                raylib::ffi::rlEnableTexture(self.shadow_map_textures[i].texture.id);
                raylib::ffi::rlSetUniform(
                    map_locks[i],
                    &(10 + i) as *const _ as *const _,
                    raylib::ffi::ShaderUniformDataType::SHADER_UNIFORM_INT as i32,
                    1,
                );
            }
        }
        let mut to_load = HashSet::new();
        for obj in scene.objects.values() {
            if !self.loaded_meshes.contains_key(&obj.model_name) {
                to_load.insert(obj.model_name.clone());
                continue;
            }
            let mesh = self.loaded_meshes.get_mut(&obj.model_name).unwrap();
            mesh.transform = obj.rotation.to_matrix().into();
            draw.draw_model(mesh, obj.position, 1.0, Color::WHITE);
        }
        self.to_load = to_load;
    }

    pub fn render_shadows(
        &mut self,
        scene: &Scene,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
        idx: GLightId,
        count: usize,
    ) -> Matrix {
        let mut draw = handle.begin_texture_mode(thread, &mut self.shadow_map_textures[count]);
        draw.clear_background(Color::WHITE);
        let cam = Camera3D::perspective(
            scene.lights[&idx].pos,
            scene.lights[&idx].direction + scene.lights[&idx].pos,
            scene.lights[&idx].up,
            scene.lights[&idx].fov,
        );
        let mut draw = draw.begin_mode3D(cam);
        unsafe {
            raylib::ffi::rlSetClipPlanes(0.01, 1000.0);
        }
        let view = unsafe { Matrix::from(raylib::ffi::rlGetMatrixModelview()) };
        let proj = unsafe { Matrix::from(raylib::ffi::rlGetMatrixProjection()) };

        let shade = self.shader.as_mut().unwrap();
        let depth_lock = shade.get_shader_location("depth");
        let cam_lock = shade.get_shader_location("cam_lock");
        shade.set_shader_value(depth_lock, 1);
        shade.set_shader_value(cam_lock, cam.position);
        let mut to_load = HashSet::new();
        for obj in scene.objects.values() {
            if !self.loaded_meshes.contains_key(&obj.model_name) {
                to_load.insert(obj.model_name.clone());
                continue;
            }
            let mesh = self.loaded_meshes.get_mut(&obj.model_name).unwrap();
            mesh.transform = obj.rotation.to_matrix().into();
            draw.draw_model(mesh, obj.position, 1.0, Color::WHITE);
        }
        self.to_load = to_load;
        view * proj
    }

    pub fn render(
        &mut self,
        scene: &Scene,
        handle: &mut RaylibHandle,
        thread: &raylib::prelude::RaylibThread,
        target: &mut RenderTexture2D,
    ) {
        if self.shader.is_none() {
            self.shader = Some(handle.load_shader(
                thread,
                Some("shaders/shadow_map_vert.glsl"),
                Some("shaders/shadowmap_frag.glsl"),
            ));
            let msh = unsafe {
                let mut msh = handle
                    .load_model_from_mesh(
                        thread,
                        raylib::models::Mesh::gen_mesh_cube(thread, 1.0, 1.0, 1.0).make_weak(),
                    )
                    .unwrap();
                (*msh.materials).shader = *self.shader.as_deref().unwrap();
                println!("{:#?}", msh.get_model_bounding_box());
                msh
            };
            self.loaded_meshes.insert("box".into(), msh);
        }
        if self.shadow_map_textures.len() < scene.lights.len() {
            for _ in self.shadow_map_textures.len()..scene.lights.len() {
                self.shadow_map_textures
                    .push(handle.load_render_texture(thread, 1024, 1024).unwrap());
            }
        }
        let mut list = Vec::new();
        let indes: Vec<GLightId> = scene.lights.keys().copied().collect();
        for (count, idx) in indes.iter().enumerate() {
            list.push(self.render_shadows(scene, handle, thread, *idx, count));
        }
        self.render_scene(scene, handle, thread, &list, target);
        self.to_load.remove("box");
        for i in &self.to_load {
            let name = "models/".to_string() + i;
            let Ok(modl) = handle.load_model(thread, &name) else {
                continue;
            };
            for i in 0..modl.materialCount {
                unsafe {
                    (*modl.materials.add(i as usize)).shader = *self.shader.as_deref().unwrap();
                }
            }
            self.loaded_meshes.insert(i.clone(), modl);
        }
        self.to_load.clear();
    }
}
