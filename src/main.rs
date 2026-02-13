use raylib::{
    color::Color,
    math::{Quaternion, Vector3},
};

use crate::{
    dos::{
        DrawCall, SysHandle,
        scene::{GLight, GObject, Scene},
        setup,
    },
    state::run_text_mode,
};

pub mod dos;
pub mod id;
pub mod rtils;
pub mod state;
pub mod voip;
pub fn main() {
    //
    setup(main_func);
}
fn main_func(mut handle: SysHandle) {
    let mut scene = Scene::new();
    let light_id = scene.create_light(GLight {
        pos: Vector3::forward(),
        color: Color::WHITE,
        direction: -Vector3::forward(),
        up: Vector3::up(),
        fov: 90.0,
        casts_shadows: true,
    });
    let mesh_id = scene.create_object(GObject {
        model_name: "box".into(),
        position: Vector3::forward() * 15.0,
        rotation: Quaternion::identity(),
    });
    for i in -2..=2 {
        for j in -2..=2 {
            for k in -2..=2 {
                if i == j && j == k && k == 0 {
                    continue;
                }
                scene.create_object(GObject {
                    model_name: "box".into(),
                    position: Vector3 {
                        x: i as f32 * 5.0,
                        y: j as f32 * 5.0,
                        z: k as f32 * 5.0,
                    } - Vector3::forward() * 20.0,
                    rotation: Quaternion::identity(),
                });
            }
        }
    }
    let mut idx = 0;
    while !handle.should_exit() {
        handle.begin_drawing();
        idx += 1;
        idx = idx % 6290;
        let x = scene.get_object_mut(mesh_id).unwrap();
        x.position.x = 4.0 * (idx as f32 / 100.0).cos();
        x.position.z = 4.0 * (idx as f32 / 100.0).sin();
        //    println!("{:#?},{:#?}", x.position.x, x.position.z);
        handle.send_draw_calls(
            vec![DrawCall::DrawScene {
                start_x: 0,
                start_y: 0,
                width: 1200,
                height: 900,
                scene: scene.clone(),
            }],
            dos::Rect {
                x: 0,
                y: 0,
                w: 1200,
                h: 900,
            },
        );
        scene.camera_input(&handle);
        handle.end_drawing();
    }
}
