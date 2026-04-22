use std::sync::{Arc, Mutex};

use raylib::{
    RaylibHandle, RaylibThread,
    color::Color,
    math::{Rectangle, Vector2},
    prelude::{
        RaylibDraw, RaylibDrawHandle, RaylibScissorModeExt, RaylibShaderModeExt, RaylibTextureMode,
        RaylibTextureModeExt,
    },
    shaders::Shader,
    text::RaylibFont,
    texture::{RaylibTexture2D, RenderTexture2D, Texture2D},
};

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug)]
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
    pub fn run(self, handle: &mut RaylibHandle, thread: &RaylibThread) {
        for i in self.render_texture_calls {
            Self::run_render_cmd(i, handle, thread);
        }
        let mut draw = handle.begin_drawing(thread);
        for i in self.calls {
            Self::run_command(i, &mut draw, thread);
        }
    }

    pub fn run_fps(self, handle: &mut RaylibHandle, thread: &RaylibThread) {
        let w = handle.get_screen_width();
        for i in self.render_texture_calls {
            Self::run_render_cmd(i, handle, thread);
        }
        let mut draw = handle.begin_drawing(thread);
        for i in self.calls {
            Self::run_command(i, &mut draw, thread);
        }
        draw.draw_fps(w - 100, 100);
    }
    pub fn run_command(cmd: DrawCommand, handle: &mut RaylibDrawHandle, thread: &RaylibThread) {
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
                handle.draw_rectangle(bounds.x, bounds.y, bounds.width, bounds.height, color);
            }
            DrawCommand::DrawText {
                pos_x,
                pos_y,
                text_height,
                color,
                text,
            } => {
                handle.draw_text(&text, pos_x, pos_y, text_height, color);
            }
            DrawCommand::ClearBackground { color } => {
                handle.clear_background(color);
            }
            DrawCommand::DrawTexture {
                image,
                bounds,
                rotation,
                tint,
            } => {
                handle.draw_texture_pro(
                    &*image,
                    Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: image.width() as f32,
                        height: image.height as f32,
                    },
                    Rectangle {
                        x: bounds.x as f32,
                        y: bounds.y as f32,
                        width: bounds.width as f32,
                        height: bounds.height as f32,
                    },
                    Vector2::new(bounds.x as f32, bounds.y as f32),
                    rotation,
                    tint,
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
                    Vector2::new(bounds.x as f32, bounds.y as f32),
                    rotation,
                    tint,
                );
            }
            DrawCommand::DrawCircle { x, y, r, color } => {
                handle.draw_circle(x, y, r, color);
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
                    Vector2::new(x0 as f32, y0 as f32),
                    Vector2::new(x1 as f32, y1 as f32),
                    width,
                    color,
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
                        width,
                        color,
                    );
                }
            }
            DrawCommand::DrawPoints {
                points,
                radii,
                color,
            } => {
                for i in points {
                    handle.draw_circle(i.x, i.y, radii, color);
                }
            }
        }
    }

    pub fn run_render_cmd(
        cmd: RenderTextureCmdBuffer,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
    ) {
        let mut texture = cmd.texture.lock().unwrap();
        let mut mode = handle.begin_texture_mode(thread, &mut texture);
        for i in cmd.commands {
            Self::run_texture_draw_command(i, &mut mode, thread);
        }
    }

    pub fn run_texture_draw_command<T>(
        cmd: DrawCommand,
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
                handle.draw_rectangle(bounds.x, bounds.y, bounds.width, bounds.height, color);
            }
            DrawCommand::DrawText {
                pos_x,
                pos_y,
                text_height,
                color,
                text,
            } => {
                handle.draw_text(&text, pos_x, pos_y, text_height, color);
            }
            DrawCommand::ClearBackground { color } => {
                handle.clear_background(color);
            }
            DrawCommand::DrawTexture {
                image,
                bounds,
                rotation,
                tint,
            } => {
                handle.draw_texture_pro(
                    &*image,
                    Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: image.width() as f32,
                        height: image.height as f32,
                    },
                    Rectangle {
                        x: bounds.x as f32,
                        y: bounds.y as f32,
                        width: bounds.width as f32,
                        height: bounds.height as f32,
                    },
                    Vector2::new(bounds.x as f32, bounds.y as f32),
                    rotation,
                    tint,
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
                    Vector2::new(bounds.x as f32, bounds.y as f32),
                    rotation,
                    tint,
                );
            }
            DrawCommand::DrawCircle { x, y, r, color } => {
                handle.draw_circle(x, y, r, color);
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
                    Vector2::new(x0 as f32, y0 as f32),
                    Vector2::new(x1 as f32, y1 as f32),
                    width,
                    color,
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
                        width,
                        color,
                    );
                }
            }
            DrawCommand::DrawPoints {
                points,
                radii,
                color,
            } => {
                for i in points {
                    handle.draw_circle(i.x, i.y, radii, color);
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
pub enum Widget<'a> {
    Container {
        style: Style,
        bounds: Bounds,
        children: Vec<Widget<'a>>,
    },
    ScrollBox {
        style: Style,
        reversed: bool,
        bounds: Bounds,
        children: Vec<Widget<'a>>,
        scroll_amount: &'a mut f32,
        displacement: i32,
    },
    Text {
        style: Style,
        bounds: Bounds,
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
        child: Box<Widget<'a>>,
        bounds: Bounds,
        on_click: Box<dyn FnMut() + 'a>,
    },
}
impl<'a> Widget<'a> {
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
        }
    }

    pub fn render(
        &mut self,
        handle: &RaylibHandle,
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
                    i.render(handle, thread, cmd);
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
                    if contains {
                        **scroll_amount +=
                            -handle.get_mouse_wheel_move() * handle.get_frame_time() * 420.
                                / (displacement.abs() as f32 + 1.0)
                                * sign;
                        if **scroll_amount < 0.0 {
                            **scroll_amount = 0.0;
                        } else if **scroll_amount > 1.0 {
                            **scroll_amount = 1.0;
                        }
                    }
                    for i in children {
                        let b = i.bounds();
                        if b.intersects(bounds) {
                            i.render(handle, thread, cmd);
                        }
                    }
                });
            }
            Widget::Text {
                bounds,
                contents,
                style,
                text_height,
            } => {
                let base_x = bounds.x;
                let mut base_y = bounds.y;
                for i in contents {
                    cmd.draw_text(i.clone(), base_x, base_y, *text_height, style.text_color);
                    base_y += *text_height;
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
                child.render(handle, thread, cmd);
                if mouse_released && contains {
                    on_click();
                }
            }
        }
    }
}

pub struct ContainerBuilder<'a, 'b> {
    style: Style,
    handle: &'b RaylibHandle,
    bounds: Bounds,
    padding: i32,
    children: Vec<Widget<'a>>,
}

pub struct HorizontalContainerBuilder<'a, 'b> {
    style: Style,
    handle: &'b RaylibHandle,
    bounds: Bounds,
    padding: i32,
    children: Vec<Widget<'a>>,
}

pub struct ScrollBoxContainerBuilder<'a, 'b> {
    style: Style,
    handle: &'b RaylibHandle,
    bounds: Bounds,
    padding: i32,
    displacement: i32,
    children: Vec<Widget<'a>>,
}

pub struct ReversedScrollBoxContainerBuilder<'a, 'b> {
    style: Style,
    handle: &'b RaylibHandle,
    bounds: Bounds,
    padding: i32,
    displacement: i32,
    children: Vec<Widget<'a>>,
}
impl<'a, 'b> HorizontalContainerBuilder<'a, 'b> {
    pub fn get_style(&self) -> Style {
        self.style
    }

    pub fn new(
        handle: &'b RaylibHandle,
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
        on_click: impl FnMut() + 'a,
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

    pub fn button_image(&mut self, image: Arc<Texture2D>, width: i32, on_click: impl FnMut() + 'a) {
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

    pub fn container(&mut self, width: i32, child: impl FnOnce(&mut ContainerBuilder<'a, 'b>)) {
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
        child: impl FnOnce(&mut HorizontalContainerBuilder<'a, 'b>),
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
        scroll_amount: &'a mut f32,
        child: impl FnOnce(&mut ScrollBoxContainerBuilder<'a, 'b>),
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
        scroll_amount: &'a mut f32,
        child: impl FnOnce(&mut ReversedScrollBoxContainerBuilder<'a, 'b>),
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

    pub fn build(self) -> Widget<'a> {
        Widget::Container {
            bounds: self.bounds,
            style: self.style,
            children: self.children,
        }
    }
    pub fn with_style(
        &mut self,
        style: Style,
        to_run: impl FnOnce(&mut HorizontalContainerBuilder<'a, 'b>),
    ) {
        let old_style = self.style;
        self.style = style;
        to_run(self);
        self.style = old_style;
    }

    pub fn button_1(&mut self, text: impl AsRef<str>, width: i32, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height, width, on_click);
    }

    pub fn button_2(&mut self, text: impl AsRef<str>, width: i32, on_click: impl FnMut() + 'a) {
        self.button(
            text.as_ref(),
            self.style.paragraph_height - 4,
            width,
            on_click,
        );
    }

    pub fn button_3(&mut self, text: impl AsRef<str>, width: i32, on_click: impl FnMut() + 'a) {
        self.button(
            text.as_ref(),
            self.style.paragraph_height - 8,
            width,
            on_click,
        );
    }

    pub fn button_4(&mut self, text: impl AsRef<str>, width: i32, on_click: impl FnMut() + 'a) {
        self.button(
            text.as_ref(),
            self.style.paragraph_height - 12,
            width,
            on_click,
        );
    }
}

impl<'a, 'b> ContainerBuilder<'a, 'b> {
    pub fn new(
        handle: &'b RaylibHandle,
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

    pub fn button(&mut self, text: &str, text_height: i32, on_click: impl FnMut() + 'a) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.bounds.y + self.bounds.height + self.padding;
        let split = split_text(text, self.handle, w - self.padding * 2, text_height);
        let height = split.len() as i32 * text_height + self.padding * 2;
        self.bounds.height += height + self.padding;
        let b = Widget::Button {
            style: self.style,
            child: Box::new(Widget::Text {
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

    pub fn button_1(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height, on_click);
    }

    pub fn button_2(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height - 4, on_click);
    }

    pub fn button_3(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height - 8, on_click);
    }

    pub fn button_4(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height - 12, on_click);
    }

    pub fn button_image(&mut self, image: Arc<Texture2D>, on_click: impl FnMut() + 'a) {
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

    pub fn container(&mut self, child: impl FnOnce(&mut ContainerBuilder<'a, 'b>)) {
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
        child: impl FnOnce(&mut HorizontalContainerBuilder<'a, 'b>),
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
        scroll_amount: &'a mut f32,
        child: impl FnOnce(&mut ScrollBoxContainerBuilder<'a, 'b>),
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
        scroll_amount: &'a mut f32,
        child: impl FnOnce(&mut ReversedScrollBoxContainerBuilder<'a, 'b>),
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

    pub fn build(mut self) -> Widget<'a> {
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

    pub fn with_style(&mut self, style: Style, to_run: impl FnOnce(&mut ContainerBuilder<'a, 'b>)) {
        let old_style = self.style;
        self.style = style;
        to_run(self);
        self.style = old_style;
    }
}

impl<'a, 'b> ScrollBoxContainerBuilder<'a, 'b> {
    pub fn new(
        handle: &'b RaylibHandle,
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
        let y0 = self.displacement + self.padding;
        let split = split_text(text, self.handle, w, text_height);
        let height = split.len() as i32 * text_height;
        self.displacement += height + self.padding;
        self.children.push(Widget::Text {
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

    pub fn button(&mut self, text: &str, text_height: i32, on_click: impl FnMut() + 'a) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.displacement + self.padding;
        let split = split_text(text, self.handle, w - self.padding * 2, text_height);
        let height = split.len() as i32 * text_height + self.padding * 2;
        self.displacement += height;
        let b = Widget::Button {
            style: self.style,
            child: Box::new(Widget::Text {
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

    pub fn button_image(&mut self, image: Arc<Texture2D>, on_click: impl FnMut() + 'a) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.displacement + self.padding;
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

    pub fn container(&mut self, child: impl FnOnce(&mut ContainerBuilder<'a, 'b>)) {
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
        scroll_amount: &'a mut f32,
        height: i32,
        child: impl FnOnce(&mut ScrollBoxContainerBuilder<'a, 'b>),
    ) {
        let mut cloned = ScrollBoxContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.displacement + self.padding,
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
        scroll_amount: &'a mut f32,
        height: i32,
        child: impl FnOnce(&mut ReversedScrollBoxContainerBuilder<'a, 'b>),
    ) {
        let mut cloned = ReversedScrollBoxContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.displacement + self.padding,
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
        child: impl FnOnce(&mut HorizontalContainerBuilder<'a, 'b>),
    ) {
        let mut cloned = HorizontalContainerBuilder {
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

    pub fn image(&mut self, image: Arc<Texture2D>) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.displacement + self.padding;
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
        let y0 = self.displacement + self.padding;
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

    pub fn build(mut self, scroll_amount: &'a mut f32) -> Widget<'a> {
        let offset = ((self.displacement - self.bounds.height - self.padding * 2) as f32
            * -*scroll_amount) as i32
            + self.style.paragraph_height * 3;
        let base = Point { x: 0, y: 0 };
        let offset = Point { x: 0, y: offset };
        self.children.iter_mut().for_each(|i| i.shift(base, offset));
        Widget::ScrollBox {
            reversed: false,
            displacement: self.displacement,
            bounds: self.bounds,
            style: self.style,
            children: self.children,
            scroll_amount,
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
        to_run: impl FnOnce(&mut ScrollBoxContainerBuilder<'a, 'b>),
    ) {
        let old_style = self.style;
        self.style = style;
        to_run(self);
        self.style = old_style;
    }

    pub fn button_1(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height, on_click);
    }

    pub fn button_2(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height - 4, on_click);
    }

    pub fn button_3(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height - 8, on_click);
    }

    pub fn button_4(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height - 12, on_click);
    }
}

impl<'a, 'b> ReversedScrollBoxContainerBuilder<'a, 'b> {
    pub fn new(
        handle: &'b RaylibHandle,
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
        let y0 = self.displacement + self.padding;
        let split = split_text(text, self.handle, w, text_height);
        let height = split.len() as i32 * text_height;
        self.displacement += height + self.padding;
        self.children.push(Widget::Text {
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

    pub fn button(&mut self, text: &str, text_height: i32, on_click: impl FnMut() + 'a) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.displacement + self.padding;
        let split = split_text(text, self.handle, w - self.padding * 2, text_height);
        let height = split.len() as i32 * text_height + self.padding * 2;
        self.displacement += height;
        let b = Widget::Button {
            style: self.style,
            child: Box::new(Widget::Text {
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

    pub fn button_image(&mut self, image: Arc<Texture2D>, on_click: impl FnMut() + 'a) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.displacement + self.padding;
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

    pub fn container(&mut self, child: impl FnOnce(&mut ContainerBuilder<'a, 'b>)) {
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
        scroll_amount: &'a mut f32,
        height: i32,
        child: impl FnOnce(&mut ScrollBoxContainerBuilder<'a, 'b>),
    ) {
        let mut cloned = ScrollBoxContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.displacement + self.padding,
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
        scroll_amount: &'a mut f32,
        height: i32,
        child: impl FnOnce(&mut ReversedScrollBoxContainerBuilder<'a, 'b>),
    ) {
        let mut cloned = ReversedScrollBoxContainerBuilder {
            style: self.style,
            handle: self.handle,
            bounds: Bounds {
                x: self.bounds.x + self.padding,
                y: self.displacement + self.padding,
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
        child: impl FnOnce(&mut HorizontalContainerBuilder<'a, 'b>),
    ) {
        let mut cloned = HorizontalContainerBuilder {
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

    pub fn image(&mut self, image: Arc<Texture2D>) {
        let w = self.bounds.width - self.padding * 2;
        let x0 = self.bounds.x + self.padding;
        let y0 = self.displacement + self.padding;
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
        let y0 = self.displacement + self.padding;
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

    pub fn build(mut self, scroll_amount: &'a mut f32) -> Widget<'a> {
        let offset = ((self.displacement - self.bounds.height) as f32 * (*scroll_amount)) as i32
            + self.bounds.height
            - self.displacement
            + self.style.paragraph_height * 2
            + self.padding * 2;
        let base = Point { x: 0, y: 0 };
        let offset = Point { x: 0, y: offset };
        self.children.iter_mut().for_each(|i| i.shift(base, offset));
        Widget::ScrollBox {
            reversed: true,
            displacement: self.displacement,
            bounds: self.bounds,
            style: self.style,
            children: self.children,
            scroll_amount,
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
        to_run: impl FnOnce(&mut ReversedScrollBoxContainerBuilder<'a, 'b>),
    ) {
        let old_style = self.style;
        self.style = style;
        to_run(self);
        self.style = old_style;
    }

    pub fn button_1(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height, on_click);
    }

    pub fn button_2(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height - 4, on_click);
    }

    pub fn button_3(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height - 8, on_click);
    }

    pub fn button_4(&mut self, text: impl AsRef<str>, on_click: impl FnMut() + 'a) {
        self.button(text.as_ref(), self.style.paragraph_height - 12, on_click);
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
    let mut dw = 0;
    for i in text.chars() {
        if i == '\n' {
            current.pop();
            out.push(current.clone().into());
            current.clear();
        } else {
            current.push(i);
            let glyph = i;
            let mut s = [0u8; 8];
            let st = glyph.encode_utf8(&mut s);
            let nw = handle.measure_text(st, text_height) + dw;
            if nw >= width {
                dw = 0;
                current.pop();
                out.push(current.clone().into());
                current.clear();
                current.push(i)
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

pub struct GUI<'a> {
    actual_dimension_x: i32,
    actual_dimension_y: i32,
    widgets: Vec<Widget<'a>>,
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
            background_color: Color::WHITE,
            outline_color: Color::BLACK,
            padding: 5,
            header_height: 48,
            paragraph_height: 24,
        }
    }
}

impl<'a> GUI<'a> {
    pub fn new(handle: &'a mut RaylibHandle, thread: &'a RaylibThread) -> Self {
        let dx = handle.get_screen_width();
        let dy = handle.get_screen_height();
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
        to_run: impl FnOnce(&mut ContainerBuilder<'a, 'b>),
    ) {
        let mut x: ContainerBuilder<'a, 'b> =
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
        to_run: impl FnOnce(&mut HorizontalContainerBuilder<'a, 'b>),
    ) {
        let mut x: HorizontalContainerBuilder<'a, 'b> = HorizontalContainerBuilder::new(
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
        scroll_amount: &'a mut f32,
        to_run: impl FnOnce(&mut ScrollBoxContainerBuilder<'a, 'b>),
    ) {
        let mut x: ScrollBoxContainerBuilder<'a, 'b> = ScrollBoxContainerBuilder::new(
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

    pub fn render_commands(&mut self) -> CommandBuffer {
        let mut cmds = CommandBufferBuilder::new();
        cmds.clear_background(self.style.background_color);
        for i in &mut self.widgets {
            i.render(self.handle, self.thread, &mut cmds);
        }
        cmds.build()
    }

    pub fn render(&mut self) {
        let cmds = self.render_commands();
        cmds.run(self.handle, self.thread);
    }

    pub fn render_fps(&mut self) {
        let cmds = self.render_commands();
        cmds.run_fps(self.handle, self.thread);
    }

    pub fn centered_horizontal<'b>(
        &'b mut self,
        to_run: impl FnOnce(&mut HorizontalContainerBuilder<'a, 'b>),
    ) {
        let mut x: HorizontalContainerBuilder<'a, 'b> = HorizontalContainerBuilder::new(
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
