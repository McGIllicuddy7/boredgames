use std::collections::{HashMap, VecDeque};
use std::process::exit;
use std::sync::Arc;

use crate::dos::tilemap::{TILE_LAYER, TileMap};
use crate::id::{ArachneId, GlobalId, IdAllocator};
use crate::state::{Message, MessageContents};
use crate::voip::spawn_voip;
use crate::{
    dos::SysHandle, dos::SysUiMode, rtils::marathon::BStream, state::ServerState, voip::VoipCmd,
};
pub struct Client {
    pub state: ServerState,
}
impl Client {
    pub fn run(
        &mut self,
        handle: &mut SysHandle,
        con: &BStream<Message>,
        voip: &mut Option<BStream<VoipCmd>>,
        port: &str,
    ) {
        con.send(Message::Connect {
            username: self.state.name.clone(),
            id: GlobalId::invalid(),
        })
        .unwrap();
        let mut should_dc = false;
        let mut should_exit = false;
        while !handle.should_exit() {
            if should_dc {
                let _ = con.send(Message::Disconnect {
                    username: self.state.name.clone(),
                    id: self.state.id,
                });
                break;
            }
            if should_exit {
                let _ = con.send(Message::Disconnect {
                    username: self.state.name.clone(),
                    id: self.state.id,
                });
                exit(0);
            }
            while let Ok(Some(msg)) = con.receive() {
                ServerState::print_message(&msg);
                match msg {
                    Message::Connect { username, id } => {
                        self.state.users.insert(id, username);
                    }
                    Message::Disconnect { username: _, id } => {
                        self.state.users.remove(&id);
                    }
                    Message::HandShake {
                        state,
                        allocator_base,
                        user_id,
                    } => {
                        let name = self.state.name.clone();
                        self.state = state;
                        self.state.name = name;
                        self.state.alloc = Arc::new(IdAllocator::new(allocator_base));
                        self.state.id = user_id;
                        *voip = Some(spawn_voip(port, user_id).unwrap());
                    }
                    Message::YouShouldLeave => {
                        should_dc = true;
                    }
                    Message::UpdateMap { update } => {
                        self.state.update(update, &con);
                    }
                    Message::SetUsername { id, to } => {
                        *self.state.users.get_mut(&id).unwrap() = to;
                    }
                    Message::SendMessage(x) => self.state.messages.push_back(x),
                    Message::RequestMessagePage { indx: _, before: _ } => {}
                    Message::MessagePage {
                        start_indx: _,
                        contents: _,
                    } => {}
                }
            }
            if should_dc {
                con.send(Message::Disconnect {
                    username: self.state.name.clone(),
                    id: self.state.id,
                })
                .unwrap();
            }
            handle.begin_drawing();
            handle.begin_div(900, 900);
            self.state.map.enable_mouse();
            let (_, tmp) = self.state.map.draw(50, 50, 850, 850, handle, false);
            if tmp.is_some() {
                todo!();
            }
            handle.end_div();
            handle.begin_div(300, 900);
            let v: Vec<String> = self
                .state
                .messages
                .iter()
                .map(|m| match &m.contents {
                    MessageContents::Text { contents } => m.user_name.clone() + ":" + contents,
                    _ => {
                        todo!();
                    }
                })
                .collect();
            handle
                .draw_text_scroll_box("messages", 300, 600, 10, false, &v, |i: &String| i.clone());
            let omsg = handle.text_user_input_saved_exp("input", 0, 0, 300, 100, 10);
            if let Some(m) = omsg {
                self.state
                    .run_cmd(m, con, &mut should_dc, &mut should_exit, voip);
            }
            handle.end_div();
            handle.end_drawing();
        }
    }
}

pub fn run_client(handle: &mut SysHandle) {
    while !handle.should_exit() {
        let mut name = "bridget".to_string();
        handle.begin_drawing();
        handle.begin_div_exp(400, 200, 400, 400, true, SysUiMode::Sequential);
        let msg = handle.text_user_input_saved_exp("base_input", 0, 0, 400, 200, 10);
        handle.end_div();
        handle.end_drawing();
        if let Some(input) = msg {
            let mut inputs = input.split_ascii_whitespace();
            let Some(cmd) = inputs.next() else {
                continue;
            };
            match cmd {
                "name" => {
                    let Some(to) = inputs.next() else {
                        continue;
                    };
                    name = to.into();
                }
                "host" => {
                    let Some(to) = inputs.next() else {
                        continue;
                    };
                    let (stream, inp) = BStream::create();
                    crate::state::host_server(to, inp);
                    let state = ServerState {
                        messages_start: 0,
                        messages: VecDeque::new(),
                        users: HashMap::new(),
                        id: GlobalId::invalid(),
                        name: name.clone(),
                        map: TileMap::new(100, 100, "bg.png".to_string(), Vec::new()),
                        alloc: Arc::new(IdAllocator::new(0)),
                        current_layer: TILE_LAYER,
                    };
                    let mut client = Client { state };
                    client.run(handle, &stream, &mut None, to);
                }
                "join" => {
                    let Some(to) = inputs.next() else {
                        continue;
                    };
                    let Ok(stream) = std::net::TcpStream::connect((to, 8008)) else {
                        continue;
                    };
                    let state = ServerState {
                        messages_start: 0,
                        messages: VecDeque::new(),
                        users: HashMap::new(),
                        id: GlobalId::invalid(),
                        name: name.clone(),
                        map: TileMap::new(100, 100, "bg.png".to_string(), Vec::new()),
                        current_layer: TILE_LAYER,
                        alloc: Arc::new(IdAllocator::new(0)),
                    };
                    let st = BStream::from_stream(stream);
                    let mut client = Client { state };
                    client.run(handle, &st, &mut None, to);
                }
                "exit" => {
                    break;
                }
                _ => {}
            }
        }
    }
}
