use std::{collections::HashMap, net::{TcpListener, TcpStream}, thread::JoinHandle};

use crate::{state::{BPipe, GlobalIdManager, Id, IdAllocator, Message, MessageData, MessageMetaData, State, Throw, Throws,Exception, stream_read_bytes_blocking, stream_try_read_bytes, stream_write_bytes}, try_catch};

enum MessageWriter{
    Stream{s:TcpStream}, 
    Pipe{p:BPipe<Message>}
}
pub struct MessagePipe{
    write:MessageWriter,
}
impl MessagePipe{
    pub fn from_stream(stream:TcpStream)->Self{
        Self { write: MessageWriter::Stream { s: stream } }
    }
    pub fn from_pipe(pipe:BPipe<Message>)->Self{
        Self { write: MessageWriter::Pipe { p: pipe } }
    }
    pub fn write_message(&mut self, message:Message)->Throws<()>{
        match &mut self.write{
            MessageWriter::Pipe { p }=>{
                p.send(message)?;
            }
            MessageWriter::Stream { s }=>{
                let srs = rmp_serde::to_vec(&message)?;
                stream_write_bytes(s, &srs)?;
            }
        }
        Ok(())
    }

    pub fn try_read_message(&mut self)->Throws<Option<Message>>{
        match &mut self.write{
            MessageWriter::Pipe { p }=>{
                p.recieve()
            }
            MessageWriter::Stream { s }=>{
                let msg = stream_try_read_bytes(s)?;
                let Some(t) = msg else{
                    return Ok(None);
                };
                let js:Message =rmp_serde::from_slice(&t)?;
                Ok(Some(js))
            }
        }
    }

    pub fn read_message_blocking(&mut self)->Throws<Message>{
        match &mut self.write{
            MessageWriter::Pipe { p }=>{
                p.recieve_wait()
            }
            MessageWriter::Stream { s }=>{
                let msg = stream_read_bytes_blocking(s)?;
                let js:Message = rmp_serde::from_slice(&msg)?;
                Ok(js)
            }
        }
    }

    pub fn read_all_available_messages(&mut self)->Throws<Vec<Message>>{
        let mut vec = Vec::new();
        while let Some(x) = self.try_read_message()?{
            vec.push(x);
        }
        Ok(vec)
    }
}

impl  Iterator for MessagePipe{
        type Item = Throws<Message>;
        fn next(&mut self) -> Option<Self::Item> {
            let tmp = self.try_read_message();
            match tmp{
                Err(e)=>{
                    Some(Err(e))
                }
                Ok(x)=>{
                    match x{
                        Some(t)=>{
                            Some(Ok(t))
                        }
                        None=>{
                            None
                        }
                    }
                }
            }
        }
    }    

pub enum ServerCtl{
    Kill, LocalConnection{con:BPipe<Message>},SaveState{to:String},
}
pub struct Server{
    pub this_state:State,
    pub connections:HashMap<Id, MessagePipe>,
    pub listener:TcpListener,
    pub allocator:IdAllocator,
    pub global:GlobalIdManager,
    pub self_id:Id,
    pub ctl:BPipe<ServerCtl>,
}
impl Server{
    pub fn new(addr:String, ctl:BPipe<ServerCtl>)->Throws<Self>{
        let global = GlobalIdManager::new();
        let base =global.alloc_bloc();
        let id = IdAllocator::new(base);
        let self_id = id.alloc();
        Ok(Self { this_state: State::new(self_id), connections: HashMap::new(), listener: TcpListener::bind(addr)?, allocator: id, global: global, self_id, ctl})
    }
    pub fn run(&mut self){
        let mut should_end = false;
        self.listener.set_nonblocking(true).unwrap();
        loop{
            let mut new_connections = Vec::new();
            for i in &mut self.ctl{
                match i{
                    Err(_)=>{
                        break;
                    }
                    Ok(f)=>{
                        match f{
                            ServerCtl::Kill=>{
                                should_end = true;
                            }
                            ServerCtl::LocalConnection { con }=>{ 
                                new_connections.push(MessagePipe::from_pipe(con));
                            }
                            ServerCtl::SaveState { to:_ }=>{
                                todo!()
                            }
                            
                        }
                    }
                }
            }
            loop{
                let i = self.listener.accept();
                match i{
                    Ok((stream, _))=>{
                        new_connections.push(MessagePipe::from_stream(stream));
                    }
                    Err(e)=>{
                        match e.kind(){
                            std::io::ErrorKind::WouldBlock=>{
                                break;
                            }
                            _=>{
                                should_end = true;
                                break;
                            }
                        }
                    }
                }
            }
            let mut dropped_connections = Vec::new();
            let mut messages = Vec::new();
            for (id, con) in &mut self.connections{
                for j in con{
                    match j{
                        Err(_)=>{
                            dropped_connections.push(*id);
                        }
                        Ok(m)=>{
                            messages.push(m);
                        }
                    }
                }
            }
            let mut updates:Vec<Message> = Vec::new();
            let mut state_update_requests = Vec::new();
            for i in messages{
                match &i.data{
                    MessageData::Connect { username }=>{
                        println!("{} connected", username)
                    }
                    MessageData::Disconnect { username:_, allocator_start:_ }=>{
                        dropped_connections.push(i.meta.sender);
                    }
                    MessageData::RequestFullStateUpdate=>{
                        state_update_requests.push(i.meta.sender);
                    }
                    MessageData::FullStateUpdate { state }=>{
                        self.this_state= state.clone();
                        updates.push(i.clone());
                    }
                    _=>{
                        updates.push(i.clone());
                        let r = self.this_state.handle_messsage(i);
                        if r.is_err(){
                            println!("{:#?}", r.unwrap_err());
                        }
                    }
                }
            }
            for i in state_update_requests{
                let Some(con) = self.connections.get_mut(&i)else {
                    continue;
                };
                try_catch!(try {
                    con.write_message(Message { meta: MessageMetaData { sender: self.self_id }, data:  MessageData::FullStateUpdate { state: self.this_state.clone() }})?;
                } catch(_e){
                    dropped_connections.push(i);
                });
            }
            for i in dropped_connections{
                
            }
            if should_end{
                break;
            }
        }
    }
}
pub fn run_server(addr:String, ctl:BPipe<ServerCtl>)->Throws<JoinHandle<()>>{
    let mut serv = Server::new(addr, ctl)?;
    Ok(std::thread::spawn(move||{
        serv.run();
    }))
}