use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use raylib::{
    RaylibHandle, RaylibThread,
    color::Color,
    math::{Rectangle, Vector2},
    prelude::{
        RaylibDraw, RaylibDrawHandle, RaylibScissorModeExt, RaylibShaderModeExt, RaylibTextureMode,
        RaylibTextureModeExt,
    },
    shaders::Shader,
    texture::{RaylibTexture2D, RenderTexture2D, Texture2D},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
impl Bounds {
    pub fn contains_point(&self, point: Point) -> bool {
        let v = Rectangle::new(
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
        );
        let v2 = Vector2::new(point.x as f32, point.y as f32);
        v.check_collision_point_rec(v2)
    }

    pub fn intersects(&self, other: &Self) -> bool {
        let v = Rectangle::new(
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
        );
        let v2 = Rectangle::new(
            other.x as f32,
            other.y as f32,
            other.width as f32,
            other.height as f32,
        );
        v.check_collision_recs(&v2)
    }
}

#[derive(Clone)]
pub enum DrawCommand {
    Shader {
        shader: Arc<Mutex<Shader>>,
        commands: Vec<DrawCommand>,
    },
    MutateShader {
        shader: Arc<Mutex<Shader>>,
        function: Arc<Mutex<Box<dyn FnMut(&mut Shader)>>>,
    },
    Scissor {
        children: Vec<DrawCommand>,
        bounds: Bounds,
    },
    DrawRectangle {
        color: Color,
        bounds: Bounds,
    },
    DrawText {
        pos_x: i32,
        pos_y: i32,
        text_height: i32,
        color: Color,
        text: Arc<str>,
    },
    ClearBackground {
        color: Color,
    },
    DrawTexture {
        image: Arc<Texture2D>,
        bounds: Bounds,
        rotation: f32,
        tint: Color,
    },
    DrawRenderTexture {
        image: Arc<Mutex<RenderTexture2D>>,
        bounds: Bounds,
        rotation: f32,
        tint: Color,
    },
    DrawCircle {
        x: i32,
        y: i32,
        r: f32,
        color: Color,
    },
    DrawLine {
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        width: f32,
        color: Color,
    },
    DrawPointsLines {
        points: Vec<Point>,
        width: f32,
        color: Color,
    },
    DrawPoints {
        points: Vec<Point>,
        radii: f32,
        color: Color,
    },
}
impl DrawCommand {
    pub fn set_origin(&mut self, point: Point) {
        match self {
            DrawCommand::Shader {
                shader: _,
                commands,
            } => {
                for i in commands {
                    i.set_origin(point);
                }
            }
            DrawCommand::MutateShader {
                shader: _,
                function: _,
            } => {}
            DrawCommand::Scissor { children, bounds } => {
                bounds.x += point.x;
                bounds.y += point.y;
                for i in children {
                    i.set_origin(point);
                }
            }
            DrawCommand::DrawRectangle { color: _, bounds } => {
                bounds.x += point.x;
                bounds.y += point.y;
            }
            DrawCommand::DrawText {
                pos_x,
                pos_y,
                text_height: _,
                color: _,
                text: _,
            } => {
                *pos_x += point.x;
                *pos_y += point.y;
            }
            DrawCommand::ClearBackground { color: _ } => {}
            DrawCommand::DrawTexture {
                image: _,
                bounds,
                rotation: _,
                tint: _,
            } => {
                bounds.x += point.x;
                bounds.y += point.y;
            }
            DrawCommand::DrawRenderTexture {
                image: _,
                bounds,
                rotation: _,
                tint: _,
            } => {
                bounds.x += point.x;
                bounds.y += point.y;
            }
            DrawCommand::DrawCircle {
                x,
                y,
                r: _,
                color: _,
            } => {
                *x += point.x;
                *y += point.y;
            }
            DrawCommand::DrawLine {
                x0,
                y0,
                x1,
                y1,
                width: _,
                color: _,
            } => {
                *x0 += point.x;
                *x1 += point.x;
                *y0 += point.y;
                *y1 += point.y;
            }
            DrawCommand::DrawPointsLines {
                points,
                width: _,
                color: _,
            } => {
                for i in points {
                    i.x += point.x;
                    i.y += point.y;
                }
            }
            DrawCommand::DrawPoints {
                points,
                radii: _,
                color: _,
            } => {
                for i in points {
                    i.x += point.x;
                    i.y += point.y;
                }
            }
        }
    }

    pub fn set_scale(&mut self, scale_x: f32, scale_y: f32) {
        match self {
            DrawCommand::Shader {
                shader: _,
                commands,
            } => {
                for i in commands {
                    i.set_scale(scale_x, scale_y);
                }
            }
            DrawCommand::MutateShader {
                shader: _,
                function: _,
            } => {}
            DrawCommand::Scissor { children, bounds } => {
                bounds.x = (bounds.x as f32 * scale_x) as i32;
                bounds.y = (bounds.y as f32 * scale_y) as i32;
                bounds.width = (bounds.width as f32 * scale_x) as i32;
                bounds.height = (bounds.height as f32 * scale_y) as i32;
                for i in children {
                    i.set_scale(scale_x, scale_y);
                }
            }
            DrawCommand::DrawRectangle { color: _, bounds } => {
                bounds.x = (bounds.x as f32 * scale_x) as i32;
                bounds.y = (bounds.y as f32 * scale_y) as i32;
                bounds.width = (bounds.width as f32 * scale_x) as i32;
                bounds.height = (bounds.height as f32 * scale_y) as i32;
            }
            DrawCommand::DrawText {
                pos_x,
                pos_y,
                text_height,
                color: _,
                text: _,
            } => {
                *pos_x = (*pos_x as f32 * scale_x) as i32;
                *pos_y = (*pos_y as f32 * scale_y) as i32;
                *text_height = (*text_height as f32 * scale_y) as i32;
            }
            DrawCommand::ClearBackground { color: _ } => {}
            DrawCommand::DrawTexture {
                image: _,
                bounds,
                rotation: _,
                tint: _,
            } => {
                bounds.x = (bounds.x as f32 * scale_x) as i32;
                bounds.y = (bounds.y as f32 * scale_y) as i32;
                bounds.width = (bounds.width as f32 * scale_x) as i32;
                bounds.height = (bounds.height as f32 * scale_y) as i32;
            }
            DrawCommand::DrawRenderTexture {
                image: _,
                bounds,
                rotation: _,
                tint: _,
            } => {
                bounds.x = (bounds.x as f32 * scale_x) as i32;
                bounds.y = (bounds.y as f32 * scale_y) as i32;
                bounds.width = (bounds.width as f32 * scale_x) as i32;
                bounds.height = (bounds.height as f32 * scale_y) as i32;
            }
            DrawCommand::DrawCircle { x, y, r, color: _ } => {
                *x = (*x as f32 * scale_x) as i32;
                *y = (*y as f32 * scale_y) as i32;
                *r *= (scale_x + scale_y) / 2_f32;
            }
            DrawCommand::DrawLine {
                x0,
                y0,
                x1,
                y1,
                width: _,
                color: _,
            } => {
                *x0 = (*x0 as f32 * scale_x) as i32;
                *y0 = (*y0 as f32 * scale_y) as i32;
                *x1 = (*x1 as f32 * scale_x) as i32;
                *y1 = (*y1 as f32 * scale_y) as i32;
            }
            DrawCommand::DrawPointsLines {
                points,
                width: _,
                color: _,
            } => {
                for i in points {
                    i.x = (i.x as f32 * scale_x) as i32;
                    i.y = (i.y as f32 * scale_y) as i32;
                }
            }
            DrawCommand::DrawPoints {
                points,
                radii: _,
                color: _,
            } => {
                for i in points {
                    i.x = (i.x as f32 * scale_x) as i32;
                    i.y = (i.y as f32 * scale_y) as i32;
                }
            }
        }
    }
}
#[derive(Clone)]
pub struct CommandBuffer {
    render_texture_calls: Vec<RenderTextureCmdBuffer>,
    calls: Vec<DrawCommand>,
}

#[derive(Clone)]
pub struct RenderTextureCmdBuffer {
    texture: Arc<Mutex<RenderTexture2D>>,
    commands: Vec<DrawCommand>,
}

pub struct CommandBufferBuilder {
    values: Vec<DrawCommand>,
    render_texture_commands: Vec<RenderTextureCmdBuffer>,
}
impl Default for CommandBufferBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandBufferBuilder {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            render_texture_commands: Vec::new(),
        }
    }

    pub fn draw_rectangle(&mut self, x: i32, y: i32, width: i32, height: i32, color: Color) {
        self.values.push(DrawCommand::DrawRectangle {
            color,
            bounds: Bounds {
                x,
                y,
                width,
                height,
            },
        });
    }

    pub fn draw_text(
        &mut self,
        text: impl Into<Arc<str>>,
        x: i32,
        y: i32,
        text_height: i32,
        color: Color,
    ) {
        self.values.push(DrawCommand::DrawText {
            pos_x: x,
            pos_y: y,
            text_height,
            color,
            text: text.into(),
        });
    }

    pub fn clear_background(&mut self, color: Color) {
        self.values.push(DrawCommand::ClearBackground { color })
    }

    pub fn draw_texture(&mut self, image: &Arc<Texture2D>, x: i32, y: i32) {
        self.values.push(DrawCommand::DrawTexture {
            image: image.clone(),
            bounds: Bounds {
                x,
                y,
                width: image.width(),
                height: image.height(),
            },
            rotation: 0.0,
            tint: Color::WHITE,
        });
    }

    pub fn draw_texture_rotated(&mut self, image: &Arc<Texture2D>, x: i32, y: i32, rotation: f32) {
        self.values.push(DrawCommand::DrawTexture {
            image: image.clone(),
            bounds: Bounds {
                x,
                y,
                width: image.width(),
                height: image.height(),
            },
            rotation,
            tint: Color::WHITE,
        });
    }
    pub fn draw_texture_scaled(
        &mut self,
        image: &Arc<Texture2D>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        self.values.push(DrawCommand::DrawTexture {
            image: image.clone(),
            bounds: Bounds {
                x,
                y,
                width,
                height,
            },
            rotation: 0.0,
            tint: Color::WHITE,
        });
    }

    pub fn draw_texture_scaled_rotated(
        &mut self,
        image: &Arc<Texture2D>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        rotation: f32,
    ) {
        self.values.push(DrawCommand::DrawTexture {
            image: image.clone(),
            bounds: Bounds {
                x,
                y,
                width,
                height,
            },
            rotation,
            tint: Color::WHITE,
        });
    }

    pub fn draw_render_texture(&mut self, image: &Arc<Mutex<RenderTexture2D>>, x: i32, y: i32) {
        let t = image.lock().unwrap();
        let width = t.width();
        let height = t.height();
        self.values.push(DrawCommand::DrawRenderTexture {
            image: image.clone(),
            bounds: Bounds {
                x,
                y,
                width,
                height,
            },
            rotation: 0.0,
            tint: Color::WHITE,
        });
    }

    pub fn draw_render_texture_scaled(
        &mut self,
        image: &Arc<Mutex<RenderTexture2D>>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        self.values.push(DrawCommand::DrawRenderTexture {
            image: image.clone(),
            bounds: Bounds {
                x,
                y,
                width,
                height,
            },
            rotation: 0.0,
            tint: Color::WHITE,
        });
    }

    pub fn draw_circle(&mut self, x: i32, y: i32, r: f32, color: Color) {
        self.values.push(DrawCommand::DrawCircle { x, y, r, color });
    }

    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, width: f32, color: Color) {
        self.values.push(DrawCommand::DrawLine {
            x0,
            y0,
            x1,
            y1,
            width,
            color,
        });
    }

    pub fn draw_lines(&mut self, points: impl Into<Vec<Point>>, width: f32, color: Color) {
        self.values.push(DrawCommand::DrawPointsLines {
            points: points.into(),
            width,
            color,
        })
    }

    pub fn draw_points(&mut self, points: impl Into<Vec<Point>>, radii: f32, color: Color) {
        self.values.push(DrawCommand::DrawPoints {
            points: points.into(),
            radii,
            color,
        })
    }

    pub fn scissor<T>(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        to_run: impl FnOnce(&mut CommandBufferBuilder) -> T,
    ) -> T {
        let mut tmp = CommandBufferBuilder::new();
        let out = to_run(&mut tmp);
        for i in tmp.render_texture_commands {
            self.render_texture_commands.push(i);
        }
        let cmd = DrawCommand::Scissor {
            children: tmp.values,
            bounds: Bounds {
                x,
                y,
                width,
                height,
            },
        };
        self.values.push(cmd);
        out
    }

    pub fn render_texture_mode<T>(
        &mut self,
        target: &Arc<Mutex<RenderTexture2D>>,
        to_run: impl FnOnce(&mut CommandBufferBuilder) -> T,
    ) -> T {
        let mut tmp = CommandBufferBuilder::new();
        let out = to_run(&mut tmp);
        for i in tmp.render_texture_commands {
            self.render_texture_commands.push(i);
        }
        let cmds = RenderTextureCmdBuffer {
            texture: target.clone(),
            commands: tmp.values,
        };
        self.render_texture_commands.push(cmds);
        out
    }

    pub fn build(self) -> CommandBuffer {
        CommandBuffer {
            render_texture_calls: self.render_texture_commands,
            calls: self.values,
        }
    }

    pub fn run_command_buffer(&mut self, commands: CommandBuffer) {
        for i in commands.calls {
            self.values.push(i);
        }
        for j in commands.render_texture_calls {
            self.render_texture_commands.push(j);
        }
    }

    pub fn shader<T>(
        &mut self,
        shader: &Arc<Mutex<Shader>>,
        to_run: impl FnOnce(&mut CommandBufferBuilder) -> T,
    ) -> T {
        let mut tmp = CommandBufferBuilder::new();
        let out = to_run(&mut tmp);
        for i in tmp.render_texture_commands {
            self.render_texture_commands.push(i);
        }
        let cmd = DrawCommand::Shader {
            shader: shader.clone(),
            commands: tmp.values,
        };
        self.values.push(cmd);
        out
    }

    pub fn update_shader(
        &mut self,
        shader: &Arc<Mutex<Shader>>,
        to_run: impl FnMut(&mut Shader) + 'static,
    ) {
        self.values.push(DrawCommand::MutateShader {
            shader: shader.clone(),
            function: Arc::new(Mutex::new(Box::new(to_run))),
        });
    }
}

impl CommandBuffer {
    pub fn set_origin(&mut self, point: Point) {
        for i in &mut self.calls {
            i.set_origin(point);
        }
    }

    pub fn set_scale(&mut self, scale_x: f32, scale_y: f32) {
        for i in &mut self.calls {
            i.set_scale(scale_x, scale_y);
        }
    }
    pub fn run(&mut self, handle: &mut RaylibHandle, thread: &RaylibThread) {
        for i in &mut self.render_texture_calls {
            Self::run_render_cmd(i, handle, thread);
        }
        let mut draw = handle.begin_drawing(thread);
        for i in &mut self.calls {
            Self::run_command(i, &mut draw, thread);
        }
    }

    pub fn run_fps(&mut self, handle: &mut RaylibHandle, thread: &RaylibThread) {
        let w = handle.get_screen_width();
        for i in &mut self.render_texture_calls {
            Self::run_render_cmd(i, handle, thread);
        }
        let mut draw = handle.begin_drawing(thread);
        for i in &mut self.calls {
            Self::run_command(i, &mut draw, thread);
        }
        draw.draw_fps(w - 100, 100);
    }

    pub fn run_command(
        cmd: &mut DrawCommand,
        handle: &mut RaylibDrawHandle,
        thread: &RaylibThread,
    ) {
        match cmd {
            DrawCommand::Shader { shader, commands } => {
                let mut guard = shader.lock().unwrap();
                let mut mode = handle.begin_shader_mode(&mut guard);
                for i in commands {
                    Self::run_command(i, &mut mode, thread);
                }
            }
            DrawCommand::MutateShader { shader, function } => {
                let mut guard = shader.lock().unwrap();
                let mut func = function.lock().unwrap();
                func(&mut guard);
            }
            DrawCommand::Scissor { children, bounds } => {
                let mut mode =
                    handle.begin_scissor_mode(bounds.x, bounds.y, bounds.width, bounds.height);
                for i in children {
                    Self::run_command(i, &mut mode, thread);
                }
            }
            DrawCommand::DrawRectangle { color, bounds } => {
                handle.draw_rectangle(bounds.x, bounds.y, bounds.width, bounds.height, *color);
            }
            DrawCommand::DrawText {
                pos_x,
                pos_y,
                text_height,
                color,
                text,
            } => {
                handle.draw_text(text, *pos_x, *pos_y, *text_height, *color);
            }
            DrawCommand::ClearBackground { color } => {
                handle.clear_background(*color);
            }
            DrawCommand::DrawTexture {
                image,
                bounds,
                rotation,
                tint,
            } => {
                handle.draw_texture_pro(
                    &**image,
                    Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: image.width() as f32,
                        height: -image.height() as f32,
                    },
                    Rectangle {
                        x: bounds.x as f32 + (bounds.width / 2) as f32,
                        y: bounds.y as f32 + (bounds.height / 2) as f32,
                        width: bounds.width as f32,
                        height: bounds.height as f32,
                    },
                    Vector2::new(bounds.width as f32 / 2., bounds.height as f32 / 2.),
                    *rotation,
                    *tint,
                );
            }
            DrawCommand::DrawRenderTexture {
                image,
                bounds,
                rotation,
                tint,
            } => {
                let image = image.lock().unwrap();
                handle.draw_texture_pro(
                    &*image,
                    Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: image.width() as f32,
                        height: -image.height() as f32,
                    },
                    Rectangle {
                        x: bounds.x as f32,
                        y: bounds.y as f32,
                        width: bounds.width as f32,
                        height: bounds.height as f32,
                    },
                    Vector2::new(bounds.width as f32 / 2., bounds.height as f32 / 2.),
                    *rotation,
                    *tint,
                );
            }
            DrawCommand::DrawCircle { x, y, r, color } => {
                handle.draw_circle(*x, *y, *r, *color);
            }
            DrawCommand::DrawLine {
                x0,
                y0,
                x1,
                y1,
                width,
                color,
            } => {
                handle.draw_line_ex(
                    Vector2::new(*x0 as f32, *y0 as f32),
                    Vector2::new(*x1 as f32, *y1 as f32),
                    *width,
                    *color,
                );
            }
            DrawCommand::DrawPointsLines {
                points,
                width,
                color,
            } => {
                for i in 0..points.len() - 1 {
                    let (x0, y0) = (points[i].x, points[i].y);
                    let (x1, y1) = (points[i + 1].x, points[i + 1].y);
                    handle.draw_line_ex(
                        Vector2::new(x0 as f32, y0 as f32),
                        Vector2::new(x1 as f32, y1 as f32),
                        *width,
                        *color,
                    );
                }
            }
            DrawCommand::DrawPoints {
                points,
                radii,
                color,
            } => {
                for i in points {
                    handle.draw_circle(i.x, i.y, *radii, *color);
                }
            }
        }
    }

    pub fn run_render_cmd(
        cmd: &mut RenderTextureCmdBuffer,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
    ) {
        let mut texture = cmd.texture.lock().unwrap();
        let mut mode = handle.begin_texture_mode(thread, &mut texture);
        for i in &mut cmd.commands {
            Self::run_texture_draw_command(i, &mut mode, thread);
        }
    }

    pub fn run_texture_draw_command<T>(
        cmd: &mut DrawCommand,
        handle: &mut RaylibTextureMode<'_, T>,
        thread: &RaylibThread,
    ) {
        match cmd {
            DrawCommand::Shader { shader, commands } => {
                let mut guard = shader.lock().unwrap();

                let mut mode = handle.begin_shader_mode(&mut guard);
                for i in commands {
                    Self::run_texture_draw_command(i, &mut mode, thread);
                }
            }
            DrawCommand::MutateShader { shader, function } => {
                let mut guard = shader.lock().unwrap();
                let mut func = function.lock().unwrap();
                func(&mut guard);
            }
            DrawCommand::Scissor { children, bounds } => {
                let mut mode =
                    handle.begin_scissor_mode(bounds.x, bounds.y, bounds.width, bounds.height);
                for i in children {
                    Self::run_texture_draw_command(i, &mut mode, thread);
                }
            }
            DrawCommand::DrawRectangle { color, bounds } => {
                handle.draw_rectangle(bounds.x, bounds.y, bounds.width, bounds.height, *color);
            }
            DrawCommand::DrawText {
                pos_x,
                pos_y,
                text_height,
                color,
                text,
            } => {
                handle.draw_text(text, *pos_x, *pos_y, *text_height, *color);
            }
            DrawCommand::ClearBackground { color } => {
                handle.clear_background(*color);
            }
            DrawCommand::DrawTexture {
                image,
                bounds,
                rotation,
                tint,
            } => {
                handle.draw_texture_pro(
                    &**image,
                    Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: image.width() as f32,
                        height: image.height() as f32,
                    },
                    Rectangle {
                        x: bounds.x as f32,
                        y: bounds.y as f32,
                        width: bounds.width as f32,
                        height: bounds.height as f32,
                    },
                    Vector2::new(bounds.width as f32 / 2., bounds.height as f32 / 2.),
                    *rotation,
                    *tint,
                );
            }
            DrawCommand::DrawRenderTexture {
                image,
                bounds,
                rotation,
                tint,
            } => {
                let image = image.lock().unwrap();
                handle.draw_texture_pro(
                    &*image,
                    Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: image.width() as f32,
                        height: image.height() as f32,
                    },
                    Rectangle {
                        x: bounds.x as f32,
                        y: bounds.y as f32,
                        width: bounds.width as f32,
                        height: bounds.height as f32,
                    },
                    Vector2::new(bounds.width as f32 / 2., bounds.height as f32 / 2.),
                    *rotation,
                    *tint,
                );
            }
            DrawCommand::DrawCircle { x, y, r, color } => {
                handle.draw_circle(*x, *y, *r, *color);
            }
            DrawCommand::DrawLine {
                x0,
                y0,
                x1,
                y1,
                width,
                color,
            } => {
                handle.draw_line_ex(
                    Vector2::new(*x0 as f32, *y0 as f32),
                    Vector2::new(*x1 as f32, *y1 as f32),
                    *width,
                    *color,
                );
            }
            DrawCommand::DrawPointsLines {
                points,
                width,
                color,
            } => {
                for i in 0..points.len() - 1 {
                    let (x0, y0) = (points[i].x, points[i].y);
                    let (x1, y1) = (points[i + 1].x, points[i + 1].y);
                    handle.draw_line_ex(
                        Vector2::new(x0 as f32, y0 as f32),
                        Vector2::new(x1 as f32, y1 as f32),
                        *width,
                        *color,
                    );
                }
            }
            DrawCommand::DrawPoints {
                points,
                radii,
                color,
            } => {
                for i in points {
                    handle.draw_circle(i.x, i.y, *radii, *color);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub text_color: Color,
    pub container_color: Color,
    pub button_color: Color,
    pub button_down_color: Color,
    pub background_color: Color,
    pub outline_color: Color,
    pub padding: i32,
    pub header_height: i32,
    pub paragraph_height: i32,
}

#[derive(Clone, Debug)]
pub struct ScrollBoxData {
    pub value: Rc<RefCell<f32>>,
}

#[derive(Clone, Debug)]
pub struct TextBoxData {
    pub text: Rc<RefCell<String>>,
    pub output: Rc<RefCell<Option<String>>>,
    pub is_selected: Rc<RefCell<bool>>,
}
impl Default for ScrollBoxData {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollBoxData {
    pub fn new() -> Self {
        Self {
            value: Rc::new(RefCell::new(0.0)),
        }
    }
}

impl Default for TextBoxData {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBoxData {
    pub fn new() -> Self {
        Self {
            text: Rc::new(RefCell::new(String::new())),
            output: Rc::new(RefCell::new(None)),
            is_selected: Rc::new(RefCell::new(false)),
        }
    }
    pub fn output(&self) -> Option<String> {
        self.output.borrow_mut().take()
    }
}
pub enum Widget<State> {
    Container {
        style: Style,
        bounds: Bounds,
        children: Vec<Widget<State>>,
    },
    ScrollBox {
        style: Style,
        reversed: bool,
        bounds: Bounds,
        children: Vec<Widget<State>>,
        scroll_amount: ScrollBoxData,
        displacement: i32,
    },
    Text {
        style: Style,
        bounds: Bounds,
        verbatim_contents: Arc<str>,
        contents: Vec<Arc<str>>,
        text_height: i32,
    },
    Image {
        style: Style,
        to_draw: Arc<Texture2D>,
        bounds: Bounds,
    },
    ImageMut {
        style: Style,
        to_draw: Arc<Mutex<RenderTexture2D>>,
        bounds: Bounds,
    },
    Button {
        style: Style,
        child: Box<Widget<State>>,
        bounds: Bounds,
        on_click: Box<dyn FnMut(&mut State)>,
    },
    Canvas {
        style: Style,
        bounds: Bounds,
        scale_x: f32,
        scale_y: f32,
        to_run: Arc<
            Mutex<
                Box<
                    dyn FnMut(
                        Bounds,
                        &mut State,
                        &mut CommandBufferBuilder,
                        &mut RaylibHandle,
                        &RaylibThread,
                    ),
                >,
            >,
        >,
    },
    TextInput {
        data: TextBoxData,
        split: Vec<Arc<str>>,
        bounds: Bounds,
        text_height: i32,
        style: Style,
    },
    Rectangle {
        bounds: Bounds,
        color: Color,
    },
}
impl<State> Widget<State> {
    pub fn bounds(&self) -> Bounds {
        match self {
            Widget::Container {
                style: _,
                bounds,
                children: _,
            } => *bounds,
            Widget::ScrollBox {
                style: _,
                reversed: _,
                bounds,
                children: _,
                scroll_amount: _,
                displacement: _,
            } => *bounds,
            Widget::Text {
                style: _,
                verbatim_contents: _,
                bounds,
                contents: _,
                text_height: _,
            } => *bounds,
            Widget::Image {
                style: _,
                to_draw: _,
                bounds,
            } => *bounds,
            Widget::ImageMut {
                style: _,
                to_draw: _,
                bounds,
            } => *bounds,
            Widget::Button {
                style: _,
                child: _,
                bounds,
                on_click: _,
            } => *bounds,
            Widget::Canvas {
                bounds,
                to_run: _,
                style: _,
                scale_x: _,
                scale_y: _,
            } => *bounds,
            Widget::TextInput {
                data: _,
                split: _,
                bounds,
                text_height: _,
                style: _,
            } => *bounds,
            Widget::Rectangle { bounds, color: _ } => *bounds,
        }
    }
    pub fn shift(&mut self, old_pos: Point, new_pos: Point) {
        let dx = new_pos.x - old_pos.x;
        let dy = new_pos.y - old_pos.y;
        match self {
            Widget::Container {
                bounds,
                children,
                style: _,
            } => {
                bounds.x += dx;
                bounds.y += dy;
                children.iter_mut().for_each(|i| i.shift(old_pos, new_pos));
            }
            Widget::ScrollBox {
                reversed: _,
                bounds,
                children,
                scroll_amount: _,
                style: _,
                displacement: _,
            } => {
                bounds.x += dx;
                bounds.y += dy;
                children.iter_mut().for_each(|i| i.shift(old_pos, new_pos));
            }
            Widget::Text {
                bounds,
                verbatim_contents: _,
                contents: _,
                text_height: _,
                style: _,
            } => {
                bounds.x += dx;
                bounds.y += dy;
            }
            Widget::Image {
                to_draw: _,
                bounds,
                style: _,
            } => {
                bounds.x += dx;
                bounds.y += dy;
            }
            Widget::ImageMut {
                to_draw: _,
                bounds,
                style: _,
            } => {
                bounds.x += dx;
                bounds.y += dy;
            }
            Widget::Button {
                child,
                bounds,
                on_click: _,
                style: _,
            } => {
                bounds.x += dx;
                bounds.y += dy;
                child.shift(old_pos, new_pos);
            }
            Widget::Canvas {
                bounds,
                to_run: _,
                style: _,
                scale_x: _,
                scale_y: _,
            } => {
                bounds.x += dx;
                bounds.y += dy;
            }
            Widget::TextInput {
                split: _,
                bounds,
                text_height: _,
                style: _,
                data: _,
            } => {
                bounds.x += dx;
                bounds.y += dy;
            }
            Widget::Rectangle { bounds, color: _ } => {
                bounds.x += dx;
                bounds.y += dy;
            }
        }
    }

    pub fn rescale(&mut self, new_width: i32, new_height: i32) {
        let rx = (new_width as f32) / (1920.);
        let ry = (new_height as f32) / (1080.);
        match self {
            Widget::Container {
                bounds,
                children,
                style: _,
            } => {
                bounds.x = (bounds.x as f32 * rx) as i32;
                bounds.y = (bounds.y as f32 * ry) as i32;
                bounds.height = (bounds.height as f32 * ry) as i32;
                bounds.width = (bounds.width as f32 * rx) as i32;
                children
                    .iter_mut()
                    .for_each(|i| i.rescale(new_width, new_height));
            }

            Widget::ScrollBox {
                reversed: _,
                bounds,
                children,
                scroll_amount: _,
                style: _,
                displacement: _,
            } => {
                bounds.x = (bounds.x as f32 * rx) as i32;
                bounds.y = (bounds.y as f32 * ry) as i32;
                bounds.height = (bounds.height as f32 * ry) as i32;
                bounds.width = (bounds.width as f32 * rx) as i32;
                children
                    .iter_mut()
                    .for_each(|i| i.rescale(new_width, new_height));
            }

            Widget::Text {
                bounds,
                contents: _,
                verbatim_contents: _,
                text_height,
                style: _,
            } => {
                bounds.x = (bounds.x as f32 * rx) as i32;
                bounds.y = (bounds.y as f32 * ry) as i32;
                bounds.height = (bounds.height as f32 * ry) as i32;
                bounds.width = (bounds.width as f32 * rx) as i32;
                *text_height = (*text_height as f32 * ry) as i32;
            }

            Widget::Image {
                to_draw: _,
                bounds,
                style: _,
            } => {
                bounds.x = (bounds.x as f32 * rx) as i32;
                bounds.y = (bounds.y as f32 * ry) as i32;
                bounds.height = (bounds.height as f32 * ry) as i32;
                bounds.width = (bounds.width as f32 * rx) as i32;
            }

            Widget::ImageMut {
                to_draw: _,
                bounds,
                style: _,
            } => {
                bounds.x = (bounds.x as f32 * rx) as i32;
                bounds.y = (bounds.y as f32 * ry) as i32;
                bounds.height = (bounds.height as f32 * ry) as i32;
                bounds.width = (bounds.width as f32 * rx) as i32;
            }

            Widget::Button {
                child,
                bounds,
                on_click: _,
                style: _,
            } => {
                bounds.x = (bounds.x as f32 * rx) as i32;
                bounds.y = (bounds.y as f32 * ry) as i32;
                bounds.height = (bounds.height as f32 * ry) as i32;
                bounds.width = (bounds.width as f32 * rx) as i32;
                child.rescale(new_width, new_height);
            }
            Widget::Canvas {
                bounds,
                to_run: _,
                style: _,
                scale_x,
                scale_y,
            } => {
                bounds.x = (bounds.x as f32 * rx) as i32;
                bounds.y = (bounds.y as f32 * ry) as i32;
                *scale_x *= rx;
                *scale_y *= ry;
                bounds.height = (bounds.height as f32 * ry) as i32;
                bounds.width = (bounds.width as f32 * rx) as i32;
            }
            Widget::TextInput {
                split: _,
                bounds,
                text_height,
                style: _,
                data: _,
            } => {
                bounds.x = (bounds.x as f32 * rx) as i32;
                bounds.y = (bounds.y as f32 * ry) as i32;
                bounds.height = (bounds.height as f32 * ry) as i32;
                bounds.width = (bounds.width as f32 * rx) as i32;
                *text_height = (*text_height as f32 * ry) as i32;
            }
            Widget::Rectangle { bounds, color: _ } => {
                bounds.x = (bounds.x as f32 * rx) as i32;
                bounds.y = (bounds.y as f32 * ry) as i32;
                bounds.height = (bounds.height as f32 * ry) as i32;
                bounds.width = (bounds.width as f32 * rx) as i32;
            }
        }
    }

    pub fn render(
        &mut self,
        state: &mut State,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
        cmd: &mut CommandBufferBuilder,
    ) {
        match self {
            Widget::Container {
                style,
                bounds,
                children,
            } => {
                cmd.draw_rectangle(
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    style.outline_color,
                );
                cmd.draw_rectangle(
                    bounds.x + 2,
                    bounds.y + 2,
                    bounds.width - 4,
                    bounds.height - 4,
                    style.container_color,
                );
                for i in children {
                    i.render(state, handle, thread, cmd);
                }
            }
            Widget::ScrollBox {
                reversed,
                displacement,
                bounds,
                children,
                scroll_amount,
                style,
            } => {
                cmd.scissor(bounds.x, bounds.y, bounds.width, bounds.height, |cmd| {
                    cmd.draw_rectangle(
                        bounds.x,
                        bounds.y,
                        bounds.width,
                        bounds.height,
                        style.outline_color,
                    );
                    cmd.draw_rectangle(
                        bounds.x + 2,
                        bounds.y + 2,
                        bounds.width - 4,
                        bounds.height - 4,
                        style.container_color,
                    );
                    let pos = handle.get_mouse_position();
                    let contains = bounds.contains_point(Point {
                        x: pos.x as i32,
                        y: pos.y as i32,
                    });
                    let sign = if *reversed { -1. } else { 1.0 };
                    let mut scroll_amount = scroll_amount.value.borrow_mut();
                    if contains {
                        *scroll_amount +=
                            -handle.get_mouse_wheel_move() * handle.get_frame_time() * 420.
                                / (displacement.abs() as f32 + 1.0)
                                * sign;
                        if *scroll_amount < 0.0 {
                            *scroll_amount = 0.0;
                        } else if *scroll_amount > 1.0 {
                            *scroll_amount = 1.0;
                        }
                    }
                    for i in children {
                        let b = i.bounds();
                        if b.intersects(bounds) {
                            i.render(state, handle, thread, cmd);
                        }
                    }
                });
            }
            Widget::Text {
                bounds,
                contents,
                verbatim_contents,
                style,

                text_height,
            } => {
                let pos = handle.get_mouse_position();
                let mouse_released =
                    handle.is_mouse_button_released(raylib::ffi::MouseButton::MOUSE_BUTTON_RIGHT);
                let base_x = bounds.x;
                let mut base_y = bounds.y;
                for i in contents {
                    cmd.draw_text(i.clone(), base_x, base_y, *text_height, style.text_color);
                    base_y += *text_height;
                }
                if bounds.contains_point(Point {
                    x: pos.x as i32,
                    y: pos.y as i32,
                }) && (mouse_released
                    || ((handle.is_key_down(raylib::ffi::KeyboardKey::KEY_LEFT_CONTROL)
                        || handle.is_key_down(raylib::ffi::KeyboardKey::KEY_LEFT_SUPER))
                        && handle.is_key_down(raylib::ffi::KeyboardKey::KEY_C)))
                {
                    let _ = handle.set_clipboard_text(verbatim_contents);
                }
            }
            Widget::Image {
                to_draw,
                bounds,
                style: _,
            } => {
                cmd.draw_texture_scaled(to_draw, bounds.x, bounds.y, bounds.width, bounds.height);
            }
            Widget::ImageMut {
                to_draw,
                bounds,
                style: _,
            } => {
                cmd.draw_render_texture_scaled(
                    to_draw,
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                );
            }
            Widget::Button {
                child,
                bounds,
                on_click,
                style,
            } => {
                let pos = handle.get_mouse_position();
                let contains = bounds.contains_point(Point {
                    x: pos.x as i32,
                    y: pos.y as i32,
                });
                let mouse_down =
                    handle.is_mouse_button_down(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT);
                let mouse_released =
                    handle.is_mouse_button_released(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT);
                cmd.draw_rectangle(
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    style.outline_color,
                );
                cmd.draw_rectangle(
                    bounds.x + 2,
                    bounds.y + 2,
                    bounds.width - 4,
                    bounds.height - 4,
                    if mouse_down && contains {
                        style.button_down_color
                    } else if contains {
                        let mut col = style.button_color;
                        if col.r >= 5 {
                            col.r -= 5;
                        }
                        if col.g >= 5 {
                            col.g -= 5;
                        }
                        if col.b >= 5 {
                            col.b -= 5;
                        }
                        col
                    } else {
                        style.button_color
                    },
                );
                child.render(state, handle, thread, cmd);
                if mouse_released && contains {
                    on_click(state);
                }
            }
            Widget::Canvas {
                bounds,
                to_run,
                style,
                scale_x,
                scale_y,
            } => {
                let mut func = to_run.lock().unwrap();
                let mut tbuffer = CommandBufferBuilder::new();
                func(*bounds, state, &mut tbuffer, handle, thread);
                let mut built = tbuffer.build();
                built.set_scale(*scale_x, *scale_y);
                built.set_origin(Point {
                    x: bounds.x,
                    y: bounds.y,
                });
                cmd.draw_rectangle(
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    style.outline_color,
                );
                cmd.draw_rectangle(
                    bounds.x + 2,
                    bounds.y + 2,
                    bounds.width - 4,
                    bounds.height - 4,
                    style.container_color,
                );
                cmd.scissor(bounds.x, bounds.y, bounds.width, bounds.height, |cmds| {
                    cmds.run_command_buffer(built);
                });
            }
            Widget::TextInput {
                split,
                bounds,
                text_height,
                style,
                data,
            } => {
                let base_x = bounds.x + style.padding;
                let mut base_y = bounds.y + style.padding;
                let extra = (split.len() as i32 * *text_height - bounds.height)
                    .clamp(0, (split.len() as i32 * *text_height - bounds.height).abs());
                base_y -= extra;
                cmd.draw_rectangle(
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    style.outline_color,
                );
                cmd.draw_rectangle(
                    bounds.x + 2,
                    bounds.y + 2,
                    bounds.width - 4,
                    bounds.height - 4,
                    style.container_color,
                );
                let mut last_line = bounds.y + style.padding;
                let mut last_offset = style.padding;
                cmd.scissor(
                    bounds.x + 2,
                    bounds.y + 2,
                    bounds.width - 4,
                    bounds.height - 4,
                    |cmd| {
                        for i in split {
                            last_offset = handle.measure_text(&*i, *text_height);
                            last_line = base_y;
                            cmd.draw_text(
                                i.clone(),
                                base_x,
                                base_y,
                                *text_height,
                                style.text_color,
                            );
                            base_y += *text_height;
                        }
                        cmd.draw_rectangle(
                            last_offset + bounds.x + style.padding,
                            last_line,
                            *text_height / 10,
                            *text_height,
                            style.outline_color,
                        );
                    },
                );
                let pos = handle.get_mouse_position();
                let contains = bounds.contains_point(Point {
                    x: pos.x as i32,
                    y: pos.y as i32,
                });
                let mut text = data.text.borrow_mut();
                let mut is_selected = data.is_selected.borrow_mut();
                let mut output = data.output.borrow_mut();
                if contains
                    && handle.is_mouse_button_released(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT)
                {
                    *is_selected = true;
                }
                if (!contains
                    && handle.is_mouse_button_released(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT))
                    || handle.is_key_pressed(raylib::ffi::KeyboardKey::KEY_ESCAPE)
                {
                    *is_selected = false;
                }
                if *is_selected {
                    if handle.is_key_pressed(raylib::ffi::KeyboardKey::KEY_ENTER) {
                        *output = Some(text.clone());
                        text.clear();
                    }
                    if handle.is_key_pressed(raylib::ffi::KeyboardKey::KEY_DELETE)
                        || handle.is_key_pressed(raylib::ffi::KeyboardKey::KEY_BACKSPACE)
                    {
                        text.pop();
                    }
                    if let Some(c) = handle.get_char_pressed() {
                        text.push(c);
                    }
                    if (handle.is_key_down(raylib::ffi::KeyboardKey::KEY_LEFT_CONTROL)
                        || handle.is_key_down(raylib::ffi::KeyboardKey::KEY_LEFT_SUPER))
                        && handle.is_key_pressed(raylib::ffi::KeyboardKey::KEY_V)
                    {
                        if let Ok(x) = handle.get_clipboard_text() {
                            for i in x.chars() {
                                text.push(i);
                            }
                        }
                    }
                }
            }
            Widget::Rectangle { bounds, color } => {
                cmd.draw_rectangle(bounds.x, bounds.y, bounds.width, bounds.height, *color);
            }
        }
    }
}

pub struct ContainerBuilder<'a, State> {
    style: Style,
    handle: &'a RaylibHandle,
    bounds: Bounds,
    padding: i32,
    children: Vec<Widget<State>>,
}

pub struct HorizontalContainerBuilder<'a, State> {
    style: Style,
    handle: &'a RaylibHandle,
    bounds: Bounds,
    padding: i32,
    children: Vec<Widget<State>>,
}

pub struct ScrollBoxContainerBuilder<'a, State> {
    style: Style,
    handle: &'a RaylibHandle,
    bounds: Bounds,
    padding: i32,
    displacement: i32,
    children: Vec<Widget<State>>,
}

pub struct ReversedScrollBoxContainerBuilder<'a, State> {
    style: Style,
    handle: &'a RaylibHandle,
    bounds: Bounds,
    padding: i32,
    displacement: i32,
    children: Vec<Widget<State>>,
}
impl<'a, State> HorizontalContainerBuilder<'a, State> {
    pub fn get_style(&self) -> Style {
        self.style
    }

    pub fn new(
        handle: &'a RaylibHandle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        padding: i32,
        style: Style,
    ) -> Self {
        Self {
            style,
            handle,
            bounds: Bounds {
                x,
                y,
                width,
                height,
            },
            children: Vec::new(),
            padding,
        }
    }

    pub fn canvas(
        &mut self,
        width: i32,
        height: i32,
        to_render: impl FnMut(
            Bounds,
            &mut State,
            &mut CommandBufferBuilder,
            &mut RaylibHandle,
            &RaylibThread,
        ) + 'static,
    ) {
        let w = Widget::Canvas {
            bounds: Bounds {
                x: self.bounds.width + self.bounds.x + self.padding,
                y: self.bounds.y + self.padding,
                width,
                height,
            },
            scale_x: 1.0,
            scale_y: 1.0,
            style: self.style,
            to_run: Arc::new(Mutex::new(Box::new(to_render))),
        };
        self.children.push(w);
        let tmp = self.padding * 2 + height;
        if tmp > self.bounds.height {
            self.bounds.height = tmp;
        };
        self.bounds.width += width + self.padding * 2;
    }

    pub fn text(&mut self, text: &str, text_height: i32, width: i32) {
        let w = width;
        let x0 = self.bounds.x + self.bounds.width + self.padding;
        let y0 = self.bounds.y + self.padding;
        let split = split_text(text, self.handle, w, text_height);
        let height = split.len() as i32 * text_height;
        let tmp = height + self.padding * 2;
        if tmp > self.bounds.height {
            self.bounds.height = tmp;
        }
        self.bounds.width += width + self.padding * 2;
        self.children.push(Widget::Text {
            verbatim_contents: text.into(),
            style: self.style,

            text_height,
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            contents: split,
        });
    }

    pub fn h1(&mut self, text: impl AsRef<str>, width: i32) {
        self.text(text.as_ref(), self.style.header_height, width);
    }

    pub fn h2(&mut self, text: impl AsRef<str>, width: i32) {
        self.text(text.as_ref(), self.style.header_height - 4, width);
    }

    pub fn h3(&mut self, text: impl AsRef<str>, width: i32) {
        self.text(text.as_ref(), self.style.header_height - 8, width);
    }
    pub fn h4(&mut self, text: impl AsRef<str>, width: i32) {
        self.text(text.as_ref(), self.style.header_height - 12, width);
    }

    pub fn p1(&mut self, text: impl AsRef<str>, width: i32) {
        self.text(text.as_ref(), self.style.paragraph_height, width);
    }

    pub fn p2(&mut self, text: impl AsRef<str>, width: i32) {
        self.text(text.as_ref(), self.style.paragraph_height - 4, width);
    }

    pub fn p3(&mut self, text: impl AsRef<str>, width: i32) {
        self.text(text.as_ref(), self.style.paragraph_height - 8, width);
    }
    pub fn p4(&mut self, text: impl AsRef<str>, width: i32) {
        self.text(text.as_ref(), self.style.paragraph_height - 12, width);
    }

    pub fn button(
        &mut self,
        text: &str,
        text_height: i32,
        width: i32,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        let w = width;
        let x0 = self.bounds.x + self.bounds.width + self.padding;
        let y0 = self.bounds.y + self.padding;
        let split = split_text(text, self.handle, w - self.padding * 2, text_height);
        let height = split.len() as i32 * text_height + self.padding * 2;
        if height > self.bounds.height {
            self.bounds.height = height;
        }
        self.bounds.width += w + self.padding * 2;
        let b = Widget::Button {
            style: self.style,
            child: Box::new(Widget::Text {
                verbatim_contents: text.into(),
                text_height,
                style: self.style,
                bounds: Bounds {
                    x: x0 + self.padding,
                    y: y0 + self.padding,
                    width: w,
                    height: height + self.padding * 2,
                },
                contents: split,
            }),
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            on_click: Box::new(on_click),
        };
        self.children.push(b);
    }

    pub fn button_image(
        &mut self,
        image: Arc<Texture2D>,
        width: i32,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        let w = width;
        let x0 = self.bounds.x + self.padding + self.bounds.width;
        let y0 = self.bounds.y + self.padding;
        let ratio = image.width() as f32 / image.height() as f32;
        let height = ((w - self.padding * 2) as f32 * ratio) as i32;
        if height > self.bounds.height {
            self.bounds.height = height;
        }
        self.bounds.width += w + self.padding * 2;
        let b = Widget::Button {
            style: self.style,
            child: Box::new(Widget::Image {
                style: self.style,
                bounds: Bounds {
                    x: x0 + self.padding,
                    y: y0 + self.padding,
                    width: w,
                    height: height + self.padding * 2,
                },
                to_draw: image,
            }),
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            on_click: Box::new(on_click),
        };
        self.children.push(b);
    }

    pub fn container(&mut self, width: i32, child: impl FnOnce(&mut ContainerBuilder<'a, State>)) {
        let mut cloned = ContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.bounds.width + self.padding,
                y: self.bounds.y + self.padding,
                width,
                height: 0,
            },
            padding: self.padding,
            children: Vec::new(),
        };
        child(&mut cloned);
        let tmp = self.padding * 2 + cloned.bounds.height;
        if tmp > self.bounds.height {
            self.bounds.height = tmp;
        }
        self.bounds.width += width + self.padding * 2;
        self.children.push(cloned.build());
    }

    pub fn horizontal_container(
        &mut self,
        child: impl FnOnce(&mut HorizontalContainerBuilder<'a, State>),
    ) {
        let mut cloned = HorizontalContainerBuilder {
            handle: self.handle,
            style: self.style,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.padding + self.bounds.height + self.bounds.y,
                width: 0,
                height: 0,
            },
            padding: self.padding,
            children: Vec::new(),
        };
        child(&mut cloned);
        let tmp = self.padding * 2 + cloned.bounds.height;
        if tmp > self.bounds.height {
            self.bounds.height = tmp;
        };
        self.bounds.width += cloned.bounds.width + self.padding * 2;
        self.children.push(cloned.build());
    }
    pub fn scroll_box(
        &mut self,
        width: i32,
        height: i32,
        scroll_amount: &ScrollBoxData,
        child: impl FnOnce(&mut ScrollBoxContainerBuilder<'a, State>),
    ) {
        let mut cloned = ScrollBoxContainerBuilder {
            handle: self.handle,
            style: self.style,
            bounds: Bounds {
                x: self.bounds.x + self.bounds.width + self.padding,
                y: self.bounds.y + self.padding,
                width,
                height,
            },
            padding: self.padding,
            displacement: 0,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.bounds.width += self.padding * 2 + width;
        if height + self.padding * 2 > self.bounds.height {
            self.bounds.height = self.padding * 2 + height;
        }
        self.bounds.width += width + self.padding * 2;
        self.children.push(cloned.build(scroll_amount));
    }

    pub fn scroll_box_rev(
        &mut self,
        width: i32,
        height: i32,
        scroll_amount: &ScrollBoxData,
        child: impl FnOnce(&mut ReversedScrollBoxContainerBuilder<'a, State>),
    ) {
        let mut cloned = ReversedScrollBoxContainerBuilder {
            handle: self.handle,
            style: self.style,
            bounds: Bounds {
                x: self.bounds.x + self.bounds.width + self.padding,
                y: self.bounds.y + self.padding,
                width,
                height,
            },
            padding: self.padding,
            displacement: 0,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.bounds.width += self.padding * 2 + width;
        if height + self.padding * 2 > self.bounds.height {
            self.bounds.height = self.padding * 2 + height;
        }
        self.bounds.width += width + self.padding * 2;
        self.children.push(cloned.build(scroll_amount));
    }

    pub fn image(&mut self, width: i32, image: Arc<Texture2D>) {
        let w = width;
        let x0 = self.bounds.x + self.bounds.width + self.padding;
        let y0 = self.bounds.y + self.padding;
        let ratio = image.width() as f32 / image.height() as f32;
        let height = ((w) as f32 * ratio) as i32;
        let tmp = height + self.padding * 2;
        if tmp > self.bounds.height {
            self.bounds.height = tmp;
        }
        self.bounds.width += width + self.padding * 2;
        let b = Widget::Image {
            style: self.style,
            bounds: Bounds {
                x: x0 + self.padding,
                y: y0 + self.padding,
                width: w - self.padding * 2,
                height: height + self.padding * 2,
            },
            to_draw: image,
        };
        self.children.push(b);
    }

    pub fn image_mut(&mut self, width: i32, image: Arc<Mutex<RenderTexture2D>>) {
        let w = width;
        let x0 = self.bounds.x + self.bounds.width + self.padding;
        let y0 = self.bounds.y + self.padding;
        let guard = image.lock().unwrap();
        let ratio = guard.width() as f32 / guard.height() as f32;
        drop(guard);
        let height = ((w) as f32 * ratio) as i32;
        let tmp = height + self.padding * 2;
        if tmp > self.bounds.height {
            self.bounds.height = tmp;
        }
        self.bounds.width += width + self.padding * 2;
        let b = Widget::ImageMut {
            style: self.style,
            bounds: Bounds {
                x: x0 + self.padding,
                y: y0 + self.padding,
                width: w - self.padding * 2,
                height: height + self.padding * 2,
            },
            to_draw: image,
        };
        self.children.push(b);
    }

    pub fn build(self) -> Widget<State> {
        Widget::Container {
            bounds: self.bounds,
            style: self.style,
            children: self.children,
        }
    }
    pub fn with_style(
        &mut self,
        style: Style,
        to_run: impl FnOnce(&mut HorizontalContainerBuilder<'a, State>),
    ) {
        let old_style = self.style;
        self.style = style;
        to_run(self);
        self.style = old_style;
    }

    pub fn button_1(
        &mut self,
        text: impl AsRef<str>,
        width: i32,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        self.button(text.as_ref(), self.style.paragraph_height, width, on_click);
    }

    pub fn button_2(
        &mut self,
        text: impl AsRef<str>,
        width: i32,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        self.button(
            text.as_ref(),
            self.style.paragraph_height - 4,
            width,
            on_click,
        );
    }

    pub fn button_3(
        &mut self,
        text: impl AsRef<str>,
        width: i32,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        self.button(
            text.as_ref(),
            self.style.paragraph_height - 8,
            width,
            on_click,
        );
    }

    pub fn button_4(
        &mut self,
        text: impl AsRef<str>,
        width: i32,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        self.button(
            text.as_ref(),
            self.style.paragraph_height - 12,
            width,
            on_click,
        );
    }

    pub fn text_input(&mut self, text_height: i32, width: i32, height: i32, data: &TextBoxData) {
        let w = width;
        let x0 = self.bounds.x + self.bounds.width + self.padding;
        let y0 = self.bounds.y + self.padding;
        let split = split_text(&data.text.borrow(), self.handle, w, text_height);
        let tmp = height + self.padding * 2;
        if tmp > self.bounds.height {
            self.bounds.height = tmp;
        }
        self.bounds.width += width + self.padding * 2;
        self.children.push(Widget::TextInput {
            data: data.clone(),
            style: self.style,
            text_height,
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            split,
        });
    }

    pub fn rectangle(&mut self, width: i32, height: i32, color: Color) {
        let x0 = self.bounds.x + self.bounds.width + self.padding;
        let y0 = self.bounds.y + self.padding;
        let tmp = height + self.padding * 2;
        if tmp > self.bounds.height {
            self.bounds.height = tmp;
        }
        self.bounds.width += width + self.padding * 2;
        self.children.push(Widget::Rectangle {
            bounds: Bounds {
                x: x0,
                y: y0,
                width,
                height,
            },
            color,
        })
    }
}

impl<'a, State> ContainerBuilder<'a, State> {
    pub fn new(
        handle: &'a RaylibHandle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        padding: i32,
        style: Style,
    ) -> Self {
        Self {
            style,
            handle,
            bounds: Bounds {
                x,
                y,
                width,
                height,
            },
            children: Vec::new(),
            padding,
        }
    }

    pub fn get_style(&self) -> Style {
        self.style
    }

    pub fn text(&mut self, text: &str, text_height: i32) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.bounds.height + self.padding;
        let split = split_text(text, self.handle, w, text_height);
        let height = split.len() as i32 * text_height;
        self.bounds.height += height + self.padding;
        self.children.push(Widget::Text {
            verbatim_contents: text.into(),
            style: self.style,
            text_height,
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            contents: split,
        });
    }

    pub fn text_input(&mut self, data: &TextBoxData, text_height: i32, height: i32) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.bounds.height + self.padding;
        let split = split_text(&data.text.borrow(), self.handle, w, text_height);
        self.bounds.height += height + self.padding;
        self.children.push(Widget::TextInput {
            style: self.style,
            text_height,
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            split,
            data: data.clone(),
        });
    }

    pub fn button(
        &mut self,
        text: &str,
        text_height: i32,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.bounds.height + self.padding;
        let split = split_text(text, self.handle, w - self.padding * 2, text_height);
        let height = split.len() as i32 * text_height + self.padding * 2;
        self.bounds.height += height + self.padding;
        let b = Widget::Button {
            style: self.style,
            child: Box::new(Widget::Text {
                verbatim_contents: text.into(),
                style: self.style,
                text_height,
                bounds: Bounds {
                    x: x0 + self.padding,
                    y: y0 + self.padding,
                    width: w - self.padding * 2,
                    height: height + self.padding * 2,
                },
                contents: split,
            }),
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            on_click: Box::new(on_click),
        };
        self.children.push(b);
    }

    pub fn button_1(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height, on_click);
    }

    pub fn button_2(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height - 4, on_click);
    }

    pub fn button_3(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height - 8, on_click);
    }

    pub fn button_4(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height - 12, on_click);
    }

    pub fn button_image(
        &mut self,
        image: Arc<Texture2D>,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.bounds.height + self.padding;
        let ratio = image.width() as f32 / image.height() as f32;
        let height = ((w - self.padding * 2) as f32 * ratio) as i32;
        self.bounds.height += height + self.padding * 3;
        let b = Widget::Button {
            style: self.style,
            child: Box::new(Widget::Image {
                style: self.style,
                bounds: Bounds {
                    x: x0 + self.padding,
                    y: y0 + self.padding,
                    width: w - self.padding * 2,
                    height: height + self.padding * 2,
                },
                to_draw: image,
            }),
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            on_click: Box::new(on_click),
        };
        self.children.push(b);
    }

    pub fn container(&mut self, child: impl FnOnce(&mut ContainerBuilder<'a, State>)) {
        let mut cloned = ContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.bounds.y + self.bounds.height + self.padding,
                width: self.bounds.width - self.padding * 2,
                height: 0,
            },
            padding: self.padding,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.bounds.height += self.padding * 2 + cloned.bounds.height;
        self.children.push(cloned.build());
    }

    pub fn horizontal_container(
        &mut self,
        child: impl FnOnce(&mut HorizontalContainerBuilder<'a, State>),
    ) {
        let mut cloned = HorizontalContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.padding + self.bounds.height + self.bounds.y,
                width: self.bounds.width - self.padding * 2,
                height: 0,
            },
            padding: self.padding,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.bounds.height += self.padding * 2 + cloned.bounds.height;
        self.children.push(cloned.build());
    }

    pub fn scroll_box(
        &mut self,
        height: i32,
        scroll_amount: &ScrollBoxData,
        child: impl FnOnce(&mut ScrollBoxContainerBuilder<'a, State>),
    ) {
        let mut cloned = ScrollBoxContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.bounds.y + self.bounds.height + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            padding: self.padding,
            displacement: 0,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.bounds.height += height + self.padding * 2;
        self.children.push(cloned.build(scroll_amount));
    }

    pub fn scroll_box_rev(
        &mut self,
        height: i32,
        scroll_amount: &ScrollBoxData,
        child: impl FnOnce(&mut ReversedScrollBoxContainerBuilder<'a, State>),
    ) {
        let mut cloned = ReversedScrollBoxContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.bounds.y + self.bounds.height + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            padding: self.padding,
            displacement: 0,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.bounds.height += height + self.padding * 2;
        self.children.push(cloned.build(scroll_amount));
    }

    pub fn image(&mut self, image: Arc<Texture2D>) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.bounds.height + self.padding;
        let ratio = image.width() as f32 / image.height() as f32;
        let height = ((w) as f32 * ratio) as i32;
        self.bounds.height += height + self.padding;
        let b = Widget::Image {
            style: self.style,
            bounds: Bounds {
                x: x0 + self.padding,
                y: y0 + self.padding,
                width: w - self.padding * 2,
                height: height + self.padding * 2,
            },
            to_draw: image,
        };
        self.children.push(b);
    }

    pub fn image_mut(&mut self, image: Arc<Mutex<RenderTexture2D>>) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.bounds.height + self.padding;
        let guard = image.lock().unwrap();
        let ratio = guard.width() as f32 / guard.height() as f32;
        drop(guard);
        let height = ((w) as f32 * ratio) as i32;
        self.bounds.height += height + self.padding;
        let b = Widget::ImageMut {
            style: self.style,
            bounds: Bounds {
                x: x0 + self.padding,
                y: y0 + self.padding,
                width: w - self.padding * 2,
                height: height + self.padding * 2,
            },
            to_draw: image,
        };
        self.children.push(b);
    }

    pub fn build(mut self) -> Widget<State> {
        self.bounds.height += self.padding;
        Widget::Container {
            style: self.style,
            bounds: self.bounds,
            children: self.children,
        }
    }

    pub fn h1(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height);
    }

    pub fn h2(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height - 4);
    }

    pub fn h3(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height - 8);
    }
    pub fn h4(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height - 12);
    }

    pub fn p1(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height);
    }

    pub fn p2(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height - 4);
    }

    pub fn p3(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height - 8);
    }

    pub fn p4(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height - 12);
    }

    pub fn with_style(
        &mut self,
        style: Style,
        to_run: impl FnOnce(&mut ContainerBuilder<'a, State>),
    ) {
        let old_style = self.style;
        self.style = style;
        to_run(self);
        self.style = old_style;
    }

    pub fn canvas(
        &mut self,
        height: i32,
        to_render: impl FnMut(
            Bounds,
            &mut State,
            &mut CommandBufferBuilder,
            &mut RaylibHandle,
            &RaylibThread,
        ) + 'static,
    ) {
        let w = Widget::Canvas {
            bounds: Bounds {
                x: self.bounds.width + self.bounds.x + self.padding,
                y: self.bounds.y + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            scale_x: 1.0,
            scale_y: 1.0,
            style: self.style,
            to_run: Arc::new(Mutex::new(Box::new(to_render))),
        };

        self.children.push(w);
        self.bounds.height += height + self.padding * 2;
    }

    pub fn rectangle(&mut self, height: i32, color: Color) {
        let w = Widget::Rectangle {
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.bounds.y + self.bounds.height + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            color,
        };
        self.children.push(w);
        self.bounds.height += height + self.padding * 2;
    }
}

impl<'a, State> ScrollBoxContainerBuilder<'a, State> {
    pub fn new(
        handle: &'a RaylibHandle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        padding: i32,
        style: Style,
    ) -> Self {
        Self {
            style,
            handle,
            bounds: Bounds {
                x,
                y,
                width,
                height,
            },
            children: Vec::new(),
            padding,
            displacement: y,
        }
    }

    pub fn get_style(&self) -> Style {
        self.style
    }

    pub fn text(&mut self, text: &str, text_height: i32) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let split = split_text(text, self.handle, w, text_height);
        let height = split.len() as i32 * text_height;
        self.displacement += height + self.padding;
        self.children.push(Widget::Text {
            verbatim_contents: text.into(),
            style: self.style,
            text_height,
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            contents: split,
        });
    }

    pub fn text_input(&mut self, data: &TextBoxData, text_height: i32, height: i32) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let split = split_text(&data.text.borrow(), self.handle, w, text_height);
        self.displacement += height + self.padding;
        self.children.push(Widget::TextInput {
            style: self.style,
            text_height,
            data: data.clone(),
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            split,
        });
    }

    pub fn button(
        &mut self,
        text: &str,
        text_height: i32,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let split = split_text(text, self.handle, w - self.padding * 2, text_height);
        let height = split.len() as i32 * text_height + self.padding * 2;
        self.displacement += height;
        let b = Widget::Button {
            style: self.style,
            child: Box::new(Widget::Text {
                verbatim_contents: text.into(),
                style: self.style,
                text_height,
                bounds: Bounds {
                    x: x0 + self.padding,
                    y: y0 + self.padding,
                    width: w - self.padding * 2,
                    height: height + self.padding * 2,
                },
                contents: split,
            }),
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            on_click: Box::new(on_click),
        };
        self.children.push(b);
    }

    pub fn button_image(
        &mut self,
        image: Arc<Texture2D>,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let ratio = image.width() as f32 / image.height() as f32;
        let height = ((w - self.padding * 2) as f32 * ratio) as i32;
        self.displacement += height + self.padding * 2;
        let b = Widget::Button {
            style: self.style,
            child: Box::new(Widget::Image {
                style: self.style,
                bounds: Bounds {
                    x: x0 + self.padding,
                    y: y0 + self.padding,
                    width: w - self.padding * 2,
                    height: height + self.padding * 2,
                },
                to_draw: image,
            }),
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            on_click: Box::new(on_click),
        };
        self.children.push(b);
    }

    pub fn container(&mut self, child: impl FnOnce(&mut ContainerBuilder<'a, State>)) {
        let mut cloned = ContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.bounds.y + self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height: 0,
            },
            padding: self.padding,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.displacement += self.padding * 2 + cloned.bounds.height;
        self.children.push(cloned.build());
    }

    pub fn scroll_box(
        &mut self,
        scroll_amount: &ScrollBoxData,
        height: i32,
        child: impl FnOnce(&mut ScrollBoxContainerBuilder<'a, State>),
    ) {
        let mut cloned = ScrollBoxContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.bounds.y + self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            padding: self.padding,
            displacement: 0,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.displacement += self.padding * 2 + cloned.bounds.height;
        self.children.push(cloned.build(scroll_amount));
    }
    pub fn scroll_box_rev(
        &mut self,
        scroll_amount: &ScrollBoxData,
        height: i32,
        child: impl FnOnce(&mut ReversedScrollBoxContainerBuilder<'a, State>),
    ) {
        let mut cloned = ReversedScrollBoxContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.bounds.y + self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            padding: self.padding,
            displacement: 0,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.displacement += self.padding * 2 + cloned.bounds.height;
        self.children.push(cloned.build(scroll_amount));
    }

    pub fn horizontal_container(
        &mut self,
        child: impl FnOnce(&mut HorizontalContainerBuilder<'a, State>),
    ) {
        let mut cloned = HorizontalContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.bounds.y + self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height: 0,
            },
            padding: self.padding,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.displacement += self.padding * 2 + cloned.bounds.height;
        self.children.push(cloned.build());
    }

    pub fn image(&mut self, image: Arc<Texture2D>) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let ratio = image.width() as f32 / image.height() as f32;
        let height = ((w) as f32 * ratio) as i32;
        self.displacement += height + self.padding;
        let b = Widget::Image {
            style: self.style,
            bounds: Bounds {
                x: x0 + self.padding,
                y: y0 + self.padding,
                width: w - self.padding * 2,
                height: height + self.padding * 2,
            },
            to_draw: image,
        };
        self.children.push(b);
    }

    pub fn image_mut(&mut self, image: Arc<Mutex<RenderTexture2D>>) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let guard = image.lock().unwrap();
        let ratio = guard.width() as f32 / guard.height() as f32;
        drop(guard);
        let height = ((w) as f32 * ratio) as i32;
        self.displacement += height + self.padding;
        let b = Widget::ImageMut {
            style: self.style,
            bounds: Bounds {
                x: x0 + self.padding,
                y: y0 + self.padding,
                width: w - self.padding * 2,
                height: height + self.padding * 2,
            },
            to_draw: image,
        };
        self.children.push(b);
    }

    pub fn build(mut self, scroll_amount: &ScrollBoxData) -> Widget<State> {
        let offset = ((self.displacement - self.bounds.height - self.padding * 2) as f32
            * -*scroll_amount.value.borrow()) as i32;
        let base = Point { x: 0, y: 0 };
        let offset = Point { x: 0, y: offset };
        self.children.iter_mut().for_each(|i| i.shift(base, offset));
        Widget::ScrollBox {
            reversed: false,
            displacement: self.displacement,
            bounds: self.bounds,
            style: self.style,
            children: self.children,
            scroll_amount: scroll_amount.clone(),
        }
    }

    pub fn h1(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height);
    }

    pub fn h2(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height - 4);
    }

    pub fn h3(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height - 8);
    }
    pub fn h4(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height - 12);
    }

    pub fn p1(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height);
    }

    pub fn p2(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height - 4);
    }

    pub fn p3(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height - 8);
    }

    pub fn p4(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height - 12);
    }

    pub fn with_style(
        &mut self,
        style: Style,
        to_run: impl FnOnce(&mut ScrollBoxContainerBuilder<'a, State>),
    ) {
        let old_style = self.style;
        self.style = style;
        to_run(self);
        self.style = old_style;
    }

    pub fn button_1(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height, on_click);
    }

    pub fn button_2(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height - 4, on_click);
    }

    pub fn button_3(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height - 8, on_click);
    }

    pub fn button_4(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height - 12, on_click);
    }

    pub fn canvas(
        &mut self,
        height: i32,
        to_render: impl FnMut(
            Bounds,
            &mut State,
            &mut CommandBufferBuilder,
            &mut RaylibHandle,
            &RaylibThread,
        ) + 'static,
    ) {
        let w = Widget::Canvas {
            bounds: Bounds {
                x: self.bounds.width + self.bounds.x + self.padding,
                y: self.bounds.y + self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            scale_x: 1.0,
            scale_y: 1.0,
            style: self.style,
            to_run: Arc::new(Mutex::new(Box::new(to_render))),
        };
        self.children.push(w);
        self.displacement += height + self.padding * 2;
    }

    pub fn rectangle(&mut self, height: i32, color: Color) {
        let w = Widget::Rectangle {
            bounds: Bounds {
                x: self.bounds.width + self.bounds.x + self.padding,
                y: self.bounds.y + self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            color,
        };
        self.children.push(w);
        self.displacement += height + self.padding * 2;
    }
}

impl<'a, State> ReversedScrollBoxContainerBuilder<'a, State> {
    pub fn new(
        handle: &'a RaylibHandle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        padding: i32,
        style: Style,
    ) -> Self {
        Self {
            style,
            handle,
            bounds: Bounds {
                x,
                y,
                width,
                height,
            },
            children: Vec::new(),
            padding,
            displacement: y + height,
        }
    }

    pub fn get_style(&self) -> Style {
        self.style
    }

    pub fn text(&mut self, text: &str, text_height: i32) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let split = split_text(text, self.handle, w, text_height);
        let height = split.len() as i32 * text_height;
        self.displacement += height + self.padding;
        self.children.push(Widget::Text {
            verbatim_contents: text.into(),
            style: self.style,
            text_height,
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            contents: split,
        });
    }

    pub fn text_input(&mut self, data: &TextBoxData, text_height: i32, height: i32) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let split = split_text(&data.text.borrow(), self.handle, w, text_height);
        self.displacement += height + self.padding;
        self.children.push(Widget::TextInput {
            style: self.style,
            text_height,
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            split,
            data: data.clone(),
        });
    }

    pub fn button(
        &mut self,
        text: &str,
        text_height: i32,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let split = split_text(text, self.handle, w - self.padding * 2, text_height);
        let height = split.len() as i32 * text_height + self.padding * 2;
        self.displacement += height;
        let b = Widget::Button {
            style: self.style,
            child: Box::new(Widget::Text {
                verbatim_contents: text.into(),
                style: self.style,
                text_height,
                bounds: Bounds {
                    x: x0 + self.padding,
                    y: y0 + self.padding,
                    width: w - self.padding * 2,
                    height: height + self.padding * 2,
                },
                contents: split,
            }),
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            on_click: Box::new(on_click),
        };
        self.children.push(b);
    }

    pub fn button_image(
        &mut self,
        image: Arc<Texture2D>,
        on_click: impl FnMut(&mut State) + 'static,
    ) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let ratio = image.width() as f32 / image.height() as f32;
        let height = ((w - self.padding * 2) as f32 * ratio) as i32;
        self.displacement += height + self.padding * 2;
        let b = Widget::Button {
            style: self.style,
            child: Box::new(Widget::Image {
                style: self.style,
                bounds: Bounds {
                    x: x0 + self.padding,
                    y: y0 + self.padding,
                    width: w - self.padding * 2,
                    height: height + self.padding * 2,
                },
                to_draw: image,
            }),
            bounds: Bounds {
                x: x0,
                y: y0,
                width: w,
                height,
            },
            on_click: Box::new(on_click),
        };
        self.children.push(b);
    }

    pub fn container(&mut self, child: impl FnOnce(&mut ContainerBuilder<'a, State>)) {
        let mut cloned = ContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height: 0,
            },
            padding: self.padding,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.displacement += self.padding * 2 + cloned.bounds.height;
        self.children.push(cloned.build());
    }

    pub fn scroll_box(
        &mut self,
        scroll_amount: &ScrollBoxData,
        height: i32,
        child: impl FnOnce(&mut ScrollBoxContainerBuilder<'a, State>),
    ) {
        let mut cloned = ScrollBoxContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.bounds.y + self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            padding: self.padding,
            displacement: 0,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.displacement += self.padding * 2 + cloned.bounds.height;
        self.children.push(cloned.build(scroll_amount));
    }

    pub fn scroll_box_rev(
        &mut self,
        scroll_amount: &ScrollBoxData,
        height: i32,
        child: impl FnOnce(&mut ReversedScrollBoxContainerBuilder<'a, State>),
    ) {
        let mut cloned = ReversedScrollBoxContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.bounds.y + self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            padding: self.padding,
            displacement: 0,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.displacement += self.padding * 2 + cloned.bounds.height;
        self.children.push(cloned.build(scroll_amount));
    }
    pub fn horizontal_container(
        &mut self,
        child: impl FnOnce(&mut HorizontalContainerBuilder<'a, State>),
    ) {
        let mut cloned = HorizontalContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.bounds.y + self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height: 0,
            },
            padding: self.padding,
            children: Vec::new(),
        };
        child(&mut cloned);
        self.displacement += self.padding * 2 + cloned.bounds.height;
        self.children.push(cloned.build());
    }

    pub fn image(&mut self, image: Arc<Texture2D>) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let ratio = image.width() as f32 / image.height() as f32;
        let height = ((w) as f32 * ratio) as i32;
        self.displacement += height + self.padding;
        let b = Widget::Image {
            style: self.style,
            bounds: Bounds {
                x: x0 + self.padding,
                y: y0 + self.padding,
                width: w - self.padding * 2,
                height: height + self.padding * 2,
            },
            to_draw: image,
        };
        self.children.push(b);
    }

    pub fn image_mut(&mut self, image: Arc<Mutex<RenderTexture2D>>) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.displacement + self.padding;
        let guard = image.lock().unwrap();
        let ratio = guard.width() as f32 / guard.height() as f32;
        drop(guard);
        let height = ((w) as f32 * ratio) as i32;
        self.displacement += height + self.padding;
        let b = Widget::ImageMut {
            style: self.style,
            bounds: Bounds {
                x: x0 + self.padding,
                y: y0 + self.padding,
                width: w - self.padding * 2,
                height: height + self.padding * 2,
            },
            to_draw: image,
        };
        self.children.push(b);
    }

    pub fn build(mut self, scroll_amount: &ScrollBoxData) -> Widget<State> {
        let offset = ((self.displacement - self.bounds.height) as f32
            * (*scroll_amount.value.borrow())) as i32
            + self.bounds.height
            - self.displacement;
        let base = Point { x: 0, y: 0 };
        let offset = Point { x: 0, y: offset };
        self.children.iter_mut().for_each(|i| i.shift(base, offset));
        Widget::ScrollBox {
            reversed: true,
            displacement: self.displacement,
            bounds: self.bounds,
            style: self.style,
            children: self.children,
            scroll_amount: scroll_amount.clone(),
        }
    }

    pub fn h1(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height);
    }

    pub fn h2(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height - 4);
    }

    pub fn h3(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height - 8);
    }
    pub fn h4(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.header_height - 12);
    }

    pub fn p1(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height);
    }

    pub fn p2(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height - 4);
    }

    pub fn p3(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height - 8);
    }

    pub fn p4(&mut self, text: impl AsRef<str>) {
        self.text(text.as_ref(), self.style.paragraph_height - 12);
    }

    pub fn with_style(
        &mut self,
        style: Style,
        to_run: impl FnOnce(&mut ReversedScrollBoxContainerBuilder<'a, State>),
    ) {
        let old_style = self.style;
        self.style = style;
        to_run(self);
        self.style = old_style;
    }

    pub fn button_1(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height, on_click);
    }

    pub fn button_2(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height - 4, on_click);
    }

    pub fn button_3(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height - 8, on_click);
    }

    pub fn button_4(&mut self, text: impl AsRef<str>, on_click: impl FnMut(&mut State) + 'static) {
        self.button(text.as_ref(), self.style.paragraph_height - 12, on_click);
    }

    pub fn canvas(
        &mut self,
        height: i32,
        to_render: impl FnMut(
            Bounds,
            &mut State,
            &mut CommandBufferBuilder,
            &mut RaylibHandle,
            &RaylibThread,
        ) + 'static,
    ) {
        let w = Widget::Canvas {
            bounds: Bounds {
                x: self.bounds.width + self.bounds.x + self.padding,
                y: self.bounds.y + self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            to_run: Arc::new(Mutex::new(Box::new(to_render))),
            style: self.style,
            scale_x: 1.0,
            scale_y: 1.0,
        };

        self.children.push(w);
        self.displacement += height + self.padding * 2;
    }

    pub fn rectangle(&mut self, height: i32, color: Color) {
        let w = Widget::Rectangle {
            bounds: Bounds {
                x: self.bounds.width + self.bounds.x + self.padding,
                y: self.bounds.y + self.displacement + self.padding,
                width: self.bounds.width - self.padding * 2,
                height,
            },
            color,
        };
        self.children.push(w);
        self.displacement += height + self.padding * 2;
    }
}

pub fn split_text(
    text: &str,
    handle: &RaylibHandle,
    width: i32,
    text_height: i32,
) -> Vec<Arc<str>> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut tmp = String::new();
    let mut dw = 0;
    for i in text.chars() {
        if i == '\n' {
            current.pop();
            out.push(current.clone().into());
            current.clear();
        } else {
            current.push(i);
            tmp.push(i);
            tmp.push(' ');
            let nw = handle.measure_text(&tmp, text_height) + dw;
            tmp.clear();
            if nw >= width {
                let tmp2 = handle.measure_text(&current, text_height) + (text_height * 18) / 10;
                if tmp2 >= width {
                    dw = 0;
                    current.pop();
                    out.push(current.clone().into());
                    current.clear();
                    current.push(i)
                } else {
                    dw = tmp2;
                }
            } else {
                dw = nw;
            }
        }
    }
    if !current.is_empty() {
        out.push(current.into());
    }
    out
}

pub struct GUI<'a, State> {
    actual_dimension_x: i32,
    actual_dimension_y: i32,
    widgets: Vec<Widget<State>>,
    handle: &'a mut RaylibHandle,
    thread: &'a RaylibThread,
    style: Style,
}
impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

impl Style {
    pub fn new() -> Self {
        Self {
            text_color: Color::BLACK,
            container_color: Color::DARKGRAY,
            button_color: Color::GRAY,
            button_down_color: Color::DIMGRAY,
            background_color: Color::DARKGRAY,
            outline_color: Color::BLACK,
            padding: 5,
            header_height: 48,
            paragraph_height: 24,
        }
    }
}

impl<'a, State> GUI<'a, State> {
    pub fn new(_state: &State, handle: &'a mut RaylibHandle, thread: &'a RaylibThread) -> Self {
        let dx1 = handle.get_screen_width();
        let dy1 = (dx1 * 1080) / 1920;
        let dy2 = handle.get_screen_height();
        let dx2 = (dy2 * 1920) / 1080;
        let (dx, dy) = if dy1 > dy2 { (dx2, dy2) } else { (dx1, dy1) };
        Self {
            actual_dimension_x: dx,
            actual_dimension_y: dy,
            widgets: Vec::new(),
            handle,
            thread,
            style: Style::new(),
        }
    }

    pub fn container<'b>(
        &'b mut self,
        x: i32,
        y: i32,
        width: i32,
        to_run: impl FnOnce(&mut ContainerBuilder<'b, State>),
    ) {
        let mut x: ContainerBuilder<'b, State> =
            ContainerBuilder::new(self.handle, x, y, width, 0, self.style.padding, self.style);
        to_run(&mut x);
        let mut y = x.build();
        y.rescale(self.actual_dimension_x, self.actual_dimension_y);
        self.widgets.push(y);
    }

    pub fn horizontal_container<'b>(
        &'b mut self,
        x: i32,
        y: i32,
        height: i32,
        to_run: impl FnOnce(&mut HorizontalContainerBuilder<'b, State>),
    ) {
        let mut x: HorizontalContainerBuilder<'b, State> = HorizontalContainerBuilder::new(
            self.handle,
            x,
            y,
            0,
            height,
            self.style.padding,
            self.style,
        );
        to_run(&mut x);
        let mut y = x.build();
        y.rescale(self.actual_dimension_x, self.actual_dimension_y);
        self.widgets.push(y);
    }

    pub fn scroll_box<'b>(
        &'b mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        scroll_amount: &ScrollBoxData,
        to_run: impl FnOnce(&mut ScrollBoxContainerBuilder<'b, State>),
    ) {
        let mut x: ScrollBoxContainerBuilder<'b, State> = ScrollBoxContainerBuilder::new(
            self.handle,
            x,
            y,
            width,
            height,
            self.style.padding,
            self.style,
        );
        to_run(&mut x);
        let mut y = x.build(scroll_amount);
        y.rescale(self.actual_dimension_x, self.actual_dimension_y);
        self.widgets.push(y);
    }

    pub fn render_commands(&mut self, state: &mut State) -> CommandBuffer {
        let mut cmds = CommandBufferBuilder::new();
        cmds.clear_background(self.style.background_color);
        for i in &mut self.widgets {
            i.render(state, self.handle, self.thread, &mut cmds);
        }
        cmds.build()
    }

    pub fn render(&mut self, state: &mut State) {
        let mut cmds = self.render_commands(state);
        cmds.run(self.handle, self.thread);
    }

    pub fn render_fps(&mut self, state: &mut State) {
        let mut cmds = self.render_commands(state);
        cmds.run_fps(self.handle, self.thread);
    }

    pub fn centered_horizontal<'b>(
        &'b mut self,
        to_run: impl FnOnce(&mut HorizontalContainerBuilder<'b, State>),
    ) {
        let mut x: HorizontalContainerBuilder<'b, State> = HorizontalContainerBuilder::new(
            self.handle,
            0,
            0,
            0,
            0,
            self.style.padding,
            self.style,
        );
        to_run(&mut x);
        let height = x.bounds.height;
        let width = x.bounds.width;
        let delta_x = (1920 - width) / 2;
        let delta_y = (1080 - height) / 2;
        let mut y = x.build();
        y.shift(
            Point { x: 0, y: 0 },
            Point {
                x: delta_x,
                y: delta_y,
            },
        );
        y.rescale(self.actual_dimension_x, self.actual_dimension_y);
        self.widgets.push(y);
    }
}
