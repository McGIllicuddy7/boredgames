use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::poll;
use crossterm::event::read;

use crate::server::ServerCtl;
use crate::state::*;
use crate::try_catch;
use core::time;
use std::collections::HashMap;
use std::io::Stdout;
#[allow(unused)]
use std::io::{Read, Write};
use std::io::{stdin, stdout};
use std::process::exit;
pub struct TuiClient {
    pub state: Option<ClientData>,
    pub username: String,
    pub current_layer: Layer,
    pub cmd: String,
    pub prev_cmds: Vec<String>,
    pub server_cmds: Option<ServerControl>,
    pub id_table: HashMap<String, Id>,
    pub messages: Vec<(String, String)>,
    pub gui: TextFrameBuffer,
}
impl TuiClient {
    pub fn new() -> Self {
        Self {
            state: None,
            username: String::new(),
            cmd: String::new(),
            current_layer: Layer::Tokens,
            server_cmds: None,
            prev_cmds: Vec::new(),
            id_table: HashMap::new(),
            messages: Vec::new(),
            gui: TextFrameBuffer::new(),
        }
    }

    pub fn exec_msg(&mut self, msg: MessageData) -> Throws<()> {
        let Some(cd) = self.state.as_mut() else {
            return Ok(());
        };
        let message = Message {
            meta: MessageMetaData {
                sender: cd.state.self_id,
            },
            data: msg,
        };
        cd.run_cmd(message)?;
        Ok(())
    }

    pub fn run_cmd(&mut self, cmd: &str) -> Throws<()> {
        self.prev_cmds.push(cmd.into());
        let mut split = cmd.split_whitespace();
        let Some(cmd) = split.next() else {
            return Ok(());
        };
        match cmd {
            "mv" => {
                let Some(obj) = split.next() else {
                    return Ok(());
                };
                let Some(x) = split.next() else {
                    return Ok(());
                };
                let Some(y) = split.next() else {
                    return Ok(());
                };
                let None = split.next() else {
                    return Ok(());
                };
                let Some(id) = self.id_table.get(obj) else {
                    return Ok(());
                };
                let Ok(pos_x) = x.parse::<i32>() else {
                    return Ok(());
                };
                let Ok(pos_y) = y.parse::<i32>() else {
                    return Ok(());
                };
                let data = MessageData::MoveObject {
                    id: *id,
                    to: Pos { x: pos_x, y: pos_y },
                };
                self.exec_msg(data)?;
            }
            "cr" => {
                let Some(obj) = split.next() else {
                    return Ok(());
                };
                let Some(x) = split.next() else {
                    return Ok(());
                };
                let Some(y) = split.next() else {
                    return Ok(());
                };
                let None = split.next() else {
                    return Ok(());
                };
                let Ok(pos_x) = x.parse::<i32>() else {
                    return Ok(());
                };
                let Ok(pos_y) = y.parse::<i32>() else {
                    return Ok(());
                };
                let Some(x) = self.state.as_mut() else {
                    return Ok(());
                };
                let id = x.allocator.alloc();
                self.id_table.insert(obj.into(), id);
                let data = MessageData::CreateObject {
                    id,
                    obj: Object {
                        pos: Pos { x: pos_x, y: pos_y },
                        layer: self.current_layer,
                        display_name: obj.to_string(),
                        object_type: ObjectType::Token {
                            image: String::new(),
                            scale: 1,
                        },
                    },
                };
                self.exec_msg(data)?;
            }
            "destroy" => {
                let Some(obj) = split.next() else {
                    return Ok(());
                };
                let None = split.next() else {
                    return Ok(());
                };
                let Some(id) = self.id_table.get(obj) else {
                    return Ok(());
                };
                let data = MessageData::DestroyObject { id: *id };
                self.exec_msg(data)?;
            }
            "change-lyr" => {
                let Some(obj) = split.next() else {
                    return Ok(());
                };
                let None = split.next() else {
                    return Ok(());
                };
                match obj {
                    "tokens" => {
                        self.current_layer = Layer::Tokens;
                    }
                    "map" => {
                        self.current_layer = Layer::Map;
                    }
                    "gm" => {
                        self.current_layer = Layer::Gm;
                    }
                    _ => {
                        return Ok(());
                    }
                }
            }

            "mv-lyr" => {
                let Some(obj) = split.next() else {
                    return Ok(());
                };
                let Some(lyr) = split.next() else {
                    return Ok(());
                };
                let None = split.next() else {
                    return Ok(());
                };
                let lyr = match lyr {
                    "tokens" => Layer::Tokens,
                    "map" => Layer::Map,
                    "gm" => Layer::Gm,
                    _ => {
                        return Ok(());
                    }
                };
                let Some(id) = self.id_table.get(obj) else {
                    return Ok(());
                };
                let data = MessageData::ChangeObjectLayer {
                    id: *id,
                    layer: lyr,
                };
                self.exec_msg(data)?;
            }
            "help" => {
                self.prev_cmds
                    .push(format!("mv(mv obj x-coord y-coord) moves object to x,y"));
                self.prev_cmds
                    .push(format!("cr(rc obj x-coord y-coord) creates object at x,y"));
                self.prev_cmds
                    .push(format!("destroy(destroy obj) destroys object)"));
                self.prev_cmds.push(format!(
                    "change-lyr(change-lyr lyr) change selected layer to layer(one of tokens, map, gm)"
                ));
                self.prev_cmds.push(format!(
                    "mv-lyr(obj layer) moves object to layer(one of tokens, map, gm)"
                ));
                self.prev_cmds
                    .push(format!("help(help) display this message"));
                self.prev_cmds.push(format!(
                    "msg(msg message) send message(including whitespace after \"msg \")"
                ));
                self.prev_cmds
                    .push(format!("upload(upload file), upload a file"));
                self.prev_cmds
                    .push(format!("delete(delete file), delete an uploaded file"));
                self.prev_cmds
                    .push(format!("req-update(req-update) updates state by copy"));
                self.prev_cmds.push(format!(
                    "req-full-update(req-full-update) updates state by copy including images"
                ));
            }
            "msg" => {
                let Some(msg) = cmd.strip_prefix("msg ") else {
                    return Ok(());
                };
                let data = MessageData::Msg {
                    from: self.username.clone(),
                    contents: msg.into(),
                };
                self.exec_msg(data)?;
            }
            "upload" => {
                let Some(obj) = split.next() else {
                    return Ok(());
                };
                let None = split.next() else {
                    return Ok(());
                };
                let Ok(bytes) = std::fs::read(obj) else {
                    return Ok(());
                };
                let data = MessageData::ImageUpload {
                    name: obj.into(),
                    data: bytes.into(),
                };
                self.exec_msg(data)?;
            }
            "delete" => {
                let Some(obj) = split.next() else {
                    return Ok(());
                };
                let None = split.next() else {
                    return Ok(());
                };
                let data = MessageData::ImageDelete {
                    to_delete: obj.into(),
                };
                self.exec_msg(data)?;
            }
            "req-update" => {
                let None = split.next() else {
                    return Ok(());
                };
                let data = MessageData::RequestFullStateUpdate;
                self.exec_msg(data)?;
            }
            "req-full-update" => {
                let None = split.next() else {
                    return Ok(());
                };
                let data = MessageData::RequestEntireUpdate;
                self.exec_msg(data)?;
            }
            _ => {
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn update_with_client(&mut self) -> Throws<()> {
        if let Some(ste) = self.state.as_mut() {
            ste.new_frame();
        }
        while let Some(c) = self.gui.key() {
            let c = c;
            if c.is_press() {
                match c.code {
                    KeyCode::Char(c) => {
                        self.cmd.push(c);
                        while let Some((s, r)) = self.cmd.clone().split_once('\n') {
                            self.run_cmd(s)?;
                            self.cmd = r.to_string();
                        }
                    }
                    _ => {}
                }
            }
        }

        let Some(ste) = self.state.as_mut() else {
            return Ok(());
        };
        ste.update()?;
        if ste.notify_new_message {
            let msg = ste.take_new_messages();
            for i in msg {
                self.messages.push(i);
            }
        }
        let g = &mut self.gui;
        let Some(client_data) = self.state.as_ref() else {
            return Ok(());
        };
        let state = &client_data.state;
        g.begin_new_frame();
        g.draw_box(0, 0, 41, 41);
        for (_, obj) in &state.objects {
            let p = obj.pos;
            g.write_char('x', p.x + 1, p.y + 1);
        }
        g.draw_box(49, 1, 32, 52);
        g.set_bounds(50, 80, 2, 50);
        g.set_cursor(50, 2);
        for i in 0..self.messages.len() {
            let msg = format!("{}: {}", self.messages[i].0, self.messages[i].1);
            g.draw_string_wrapping(&msg);
        }
        g.draw_box(49, 51, 32, 8);
        g.set_bounds(50, 80, 52, 60);
        g.set_cursor(50, 52);
        for i in &self.prev_cmds {
            let msg = format!("{}", i);
            g.draw_string_wrapping(&msg);
        }
        g.reset_bounds();
        g.draw_box(49, 60, 32, 5);
        g.set_cursor(50, 61);
        g.draw_string("$:");
        g.draw_string_wrapping(&self.cmd);
        g.draw();
        Ok(())
    }

    pub fn run_cmd_no_client(&mut self, cmd: &str) -> Throws<()> {
        self.prev_cmds.push(cmd.to_string());
        let mut cmd = cmd.split_whitespace();
        let first = cmd.next().throw()?;
        match first {
            "host" => {
                let (ctl, ctl1) = BPipe::create();
                let h = crate::server::run_server("127.0.0.1:8080".to_string(), ctl1)?;
                let (con, con1) = BPipe::create();
                let pipe = crate::server::MessagePipe::from_pipe(con1);
                ctl.send(ServerCtl::LocalConnection { con })?;
                let Ok(data) = ClientData::new(pipe, self.username.clone()) else {
                    self.prev_cmds.push("failed to host".to_string());
                    return Ok(());
                };
                self.server_cmds = Some(ServerControl {
                    messages: ctl,
                    join_handler: h,
                });
                self.state = Some(data);
            }
            "join" => {
                let to_join = cmd.next().throw()?;
                let stream = std::net::TcpStream::connect(to_join)?;
                let pipe = crate::server::MessagePipe::from_stream(stream);
                let date = ClientData::new(pipe, self.username.clone())?;
                self.state = Some(date);
            }
            "set-username" => {
                let name = cmd.next().throw()?;
                self.username = name.to_string();
            }
            "help" => {}
            _ => {
                self.prev_cmds
                    .push(format!("error unknown command:{}", first));
            }
        }
        Ok(())
    }

    pub fn update_without_client(&mut self) -> Throws<()> {
        while let Some(c) = self.gui.key() {
            let c = c;
            if c.is_press() {
                match c.code {
                    KeyCode::Char(c) => {
                        self.cmd.push(c);
                        while let Some((s, r)) = self.cmd.clone().split_once('\n') {
                            self.run_cmd(s)?;
                            self.cmd = r.to_string();
                        }
                    }
                    _ => {}
                }
            }
        }
        let g = &mut self.gui;
        g.begin_new_frame();
        g.draw_box(49, 51, 32, 8);
        g.set_bounds(50, 80, 52, 60);
        g.set_cursor(50, 52);
        for i in &self.prev_cmds {
            let msg = format!("{}", i);
            g.draw_string_wrapping(&msg);
            g.end_line();
        }
        g.reset_bounds();
        g.draw_box(49, 60, 32, 4);
        g.set_cursor(50, 61);
        g.draw_string("$:");
        g.draw_string_wrapping(&self.cmd);
        g.draw();
        Ok(())
    }

    pub fn run(&mut self) {
        if self.state.is_some() {
            try_catch!(
                try {
                //println!("tried");
                    self.update_with_client()?;
                }
                catch(_err){
                 //   println!("help");
                    self.state = None;
                }
            );
        } else {
            let _ = self.update_without_client();
            //println!("help!");
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Char {
    pub ch: char,
    pub color_fg: u8,
    pub color_bg: u8,
}
pub struct TextFrameBuffer {
    pub carriage_x: usize,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub current_fg: u8,
    pub current_bg: u8,
    pub frame_buffer: [[Char; 100]; 65],
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

impl TextFrameBuffer {
    pub fn new() -> Self {
        Self {
            carriage_x: 0,
            cursor_x: 0,
            cursor_y: 0,
            current_fg: 10,
            current_bg: 0,
            frame_buffer: [[Char {
                ch: ' ',
                color_fg: 10,
                color_bg: 0,
            }; 100]; 65],
            min_x: 0,
            max_x: 100,
            min_y: 0,
            max_y: 65,
        }
    }

    pub fn begin_new_frame(&mut self) {
        self.console_reset();
        self.carriage_x = 0;
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.current_fg = 10;
        self.current_bg = 0;
        self.frame_buffer = [[Char {
            ch: ' ',
            color_fg: 10,
            color_bg: 0,
        }; _]; _];
    }

    //https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit
    pub fn console_select_fg_color(&mut self, color: u8) {
        write!(stdout(), "\x1b[38;5;{}m", color).unwrap();
    }

    //https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit
    pub fn console_select_bg_color(&mut self, color: u8) {
        write!(stdout(), "\x1b[48;5;{}m", color).unwrap();
    }

    pub fn console_move_cursor(&mut self, x: usize, y: usize) {
        write!(stdout(), "\x1b[{};{}H", x, y).unwrap();
    }

    pub fn console_reset_cursor(&mut self) {
        write!(stdout(), "\x1b[0J").unwrap();
        write!(stdout(), "\x1b[{};{}H", 1, 1).unwrap();
    }

    pub fn console_reset(&mut self) {
        write!(stdout(), "\x1b[0m").unwrap();
    }

    pub fn key(&mut self) -> Option<KeyEvent> {
        loop {
            let e = poll(time::Duration::from_millis(10)).unwrap();
            if e {
                let r = read().unwrap();
                match r {
                    Event::Key(c) => {
                        return Some(c);
                    }
                    _ => {
                        continue;
                    }
                }
            } else {
                return None;
            }
        }
    }

    pub fn draw(&mut self) {
        let mut current_bg = 0;
        let mut current_fg = 10;
        self.console_reset_cursor();
        self.console_select_bg_color(current_bg);
        self.console_select_fg_color(current_fg);
        for y in 0..self.frame_buffer.len() {
            for x in 0..self.frame_buffer[y].len() {
                let c = self.frame_buffer[y][x];
                if current_bg != c.color_bg {
                    current_bg = c.color_bg;
                    self.console_select_bg_color(current_bg);
                }
                if current_fg != c.color_fg {
                    current_fg = c.color_fg;
                    self.console_select_fg_color(current_fg);
                }
                write!(stdout(), "{}", c.ch).unwrap();
            }
            write!(stdout(), "\r\n").unwrap();
        }
        self.console_reset();
        stdout().flush().unwrap();
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        self.cursor_x = x as usize;
        self.cursor_y = y as usize;
        self.carriage_x = x as usize;
    }

    //https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit
    pub fn set_color(&mut self, fg: u8, bg: u8) {
        self.current_fg = fg;
        self.current_bg = bg;
    }

    pub fn end_line(&mut self) {
        self.cursor_y += 1;
        self.cursor_x = self.carriage_x;
    }

    pub fn write_char(&mut self, c: char, x: i32, y: i32) {
        if x < self.min_x || y < self.min_y || x >= self.max_x || y >= self.max_y {
            return;
        }
        self.frame_buffer[y as usize][x as usize] = Char {
            ch: c,
            color_fg: self.current_fg,
            color_bg: self.current_bg,
        };
    }

    pub fn draw_box(&mut self, x: i32, y: i32, w: i32, h: i32) {
        for i in x..x + w {
            self.write_char('-', i, y);
        }
        for i in y..y + h {
            self.write_char('|', x, i);
        }
        for i in x..x + w {
            self.write_char('-', i, y + h);
        }
        for i in y..y + h {
            self.write_char('|', x + w, i);
        }
    }

    pub fn draw_box_spaces(&mut self, x: i32, y: i32, w: i32, h: i32) {
        for i in x..x + w {
            self.write_char(' ', i, y);
        }
        for i in y..y + h {
            self.write_char(' ', x, i);
        }
        for i in x..x + w {
            self.write_char(' ', i, y + h);
        }
        for i in y..y + h {
            self.write_char(' ', x + w, i);
        }
    }

    pub fn draw_string(&mut self, string: &str) {
        for i in string.chars() {
            if i == '\n' {
                self.end_line();
            } else {
                self.write_char(i, self.cursor_x as i32, self.cursor_y as i32);
                self.cursor_x += 1;
            }
        }
    }

    pub fn draw_string_wrapping(&mut self, string: &str) {
        for i in string.chars() {
            if i == '\n' {
                self.end_line();
            } else {
                if self.cursor_x >= self.max_x as usize {
                    self.end_line();
                }
                self.write_char(i, self.cursor_x as i32, self.cursor_y as i32);
                self.cursor_x += 1;
            }
        }
    }

    pub fn set_bounds(&mut self, min_x: i32, max_x: i32, min_y: i32, max_y: i32) {
        self.min_x = min_x;
        self.max_x = max_x;
        self.min_y = min_y;
        self.max_y = max_y;
        if self.min_y < 0 {
            self.min_y = 0;
        }
        if self.min_x < 0 {
            self.min_x = 0;
        }
        if self.max_x >= self.frame_buffer[0].len() as i32 {
            self.max_x = self.frame_buffer[0].len() as i32;
        }
        if self.max_y >= self.frame_buffer.len() as i32 {
            self.max_y = self.frame_buffer.len() as i32;
        }
    }

    pub fn reset_bounds(&mut self) {
        self.min_y = 0;
        self.min_x = 0;
        self.max_x = self.frame_buffer[0].len() as i32;
        self.max_y = self.frame_buffer.len() as i32;
    }

    pub fn set_cursor(&mut self, x: i32, y: i32) {
        self.cursor_x = x as usize;
        self.cursor_y = y as usize;
        self.carriage_x = x as usize;
    }
}
