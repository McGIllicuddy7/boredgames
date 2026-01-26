use crate::{
    rtils::rtils_useful::{BPipe, stream_write_bytes},
    state::{
        Exception, Throws, stream_read_bytes_async, stream_read_bytes_blocking,
        stream_try_read_bytes,
    },
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::yield_now,
    u64,
};
pub enum BStream<T: Serialize + DeserializeOwned> {
    Stream { stream: Arc<Mutex<TcpStream>> },
    Pipe { pipe: BPipe<T> },
}

impl<T: Serialize + DeserializeOwned> BStream<T> {
    pub fn from_stream(stream: TcpStream) -> Self {
        stream.set_nonblocking(true).unwrap();
        Self::Stream {
            stream: Arc::new(Mutex::new(stream)),
        }
    }

    pub fn create() -> (Self, Self) {
        let (l1, l2) = BPipe::create();
        (Self::Pipe { pipe: l1 }, Self::Pipe { pipe: l2 })
    }

    pub fn send(&self, value: T) -> Throws<()> {
        match self {
            BStream::Stream { stream } => {
                let mut lock = stream.lock().unwrap();
                let bytes = rmp_serde::to_vec(&value).unwrap();
                stream_write_bytes(&mut lock, &bytes)
            }
            BStream::Pipe { pipe } => pipe.send(value),
        }
    }

    pub fn receive(&self) -> Throws<Option<T>> {
        match self {
            BStream::Stream { stream } => {
                let mut lock = stream.lock().unwrap();
                let Some(bytes) = stream_try_read_bytes(&mut lock)? else {
                    return Ok(None);
                };
                let x = rmp_serde::decode::from_slice::<T>(&bytes)?;
                let tmp = Ok(Some(x));
                tmp
            }
            BStream::Pipe { pipe } => pipe.recieve(),
        }
    }

    pub fn receive_wait(&self) -> Throws<T> {
        match self {
            BStream::Stream { stream } => {
                let mut lock = stream.lock().unwrap();
                let bytes = stream_read_bytes_blocking(&mut lock)?;
                let x = rmp_serde::decode::from_slice::<T>(&bytes)?;
                Ok(x)
            }
            BStream::Pipe { pipe } => pipe.recieve_wait(),
        }
    }

    pub async fn receive_async(&self) -> Throws<T> {
        match self {
            BStream::Stream { stream } => {
                let mut lock = stream.lock().unwrap();
                let out = stream_read_bytes_async(&mut lock).await?;
                let x = rmp_serde::decode::from_slice::<T>(&out)?;
                Ok(x)
            }
            BStream::Pipe { pipe } => pipe.recieve_async().await,
        }
    }
}

impl<T: Serialize + DeserializeOwned> Iterator for BStream<T> {
    type Item = Throws<T>;
    fn next(&mut self) -> Option<Self::Item> {
        let tmp = self.receive();
        match tmp {
            Err(e) => Some(Err(e)),
            Ok(x) => x.map(Ok),
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[repr(transparent)]

/*
    Thing you may want to respond to
*/
pub struct RequestId {
    inner: u64,
}
impl ArachneId for RequestId {
    fn create(x: u64) -> Self {
        Self { inner: x }
    }

    fn get(&self) -> u64 {
        self.inner
    }
}
/*
    How you get a response from something.
*/
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[repr(transparent)]
pub struct ResponseId {
    inner: u64,
}
impl ArachneId for ResponseId {
    fn create(x: u64) -> Self {
        Self { inner: x }
    }

    fn get(&self) -> u64 {
        self.inner
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message<T: Send> {
    is_response: bool,
    id: u64,
    payload: T,
}

struct ArachneControlData<T: Send + Serialize + DeserializeOwned> {
    recieved_responses: BTreeMap<ResponseId, Message<T>>,
    recieved_requests: BTreeMap<RequestId, Message<T>>,
    waiting_for: BTreeSet<ResponseId>,
    other_waiting_for: BTreeSet<RequestId>,
    buffer: VecDeque<Message<T>>,
}
pub struct Arachne<T: Send + Serialize + DeserializeOwned> {
    messages: BStream<Message<T>>,
    control: Arc<Mutex<ArachneControlData<T>>>,
}

impl<T: Send + Serialize + DeserializeOwned> Arachne<T> {
    pub fn new() -> (Self, Self) {
        let (t1, t2) = BStream::create();
        let c1 = ArachneControlData {
            recieved_responses: BTreeMap::new(),
            recieved_requests: BTreeMap::new(),
            waiting_for: BTreeSet::new(),
            other_waiting_for: BTreeSet::new(),
            buffer: VecDeque::new(),
        };
        let c2 = ArachneControlData {
            recieved_responses: BTreeMap::new(),
            recieved_requests: BTreeMap::new(),
            waiting_for: BTreeSet::new(),
            other_waiting_for: BTreeSet::new(),
            buffer: VecDeque::new(),
        };
        let s1 = Self {
            messages: t1,
            control: Arc::new(Mutex::new(c1)),
        };
        let s2 = Self {
            messages: t2,
            control: Arc::new(Mutex::new(c2)),
        };
        (s1, s2)
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        let t1 = BStream::from_stream(stream);
        let c1 = ArachneControlData {
            recieved_responses: BTreeMap::new(),
            recieved_requests: BTreeMap::new(),
            waiting_for: BTreeSet::new(),
            other_waiting_for: BTreeSet::new(),
            buffer: VecDeque::new(),
        };
        Self {
            messages: t1,
            control: Arc::new(Mutex::new(c1)),
        }
    }

    pub fn recieve(&self) -> Throws<Option<T>> {
        let mut control = self.control.lock().unwrap();
        let x = control.buffer.pop_front();
        if let Some(x) = x {
            return Ok(Some(x.payload));
        };
        while let Some(m) = self.messages.receive()? {
            if m.id != 0 {
                if m.is_response {
                    let id = ResponseId { inner: m.id };
                    if control.waiting_for.contains(&id) {
                        control.waiting_for.remove(&id);
                        control.recieved_responses.insert(id, m);
                    }
                } else {
                    let id = RequestId { inner: m.id };
                    control.other_waiting_for.insert(id);
                    control.recieved_requests.insert(id, m);
                }
            } else {
                control.buffer.push_back(m);
            }
        }
        Ok(control.buffer.pop_front().map(|i| i.payload))
    }

    pub fn recieve_request(&self) -> Throws<Option<(RequestId, T)>> {
        let mut control = self.control.lock().unwrap();
        if let Some((id, req)) = control.recieved_requests.pop_first() {
            return Ok(Some((id, req.payload)));
        }
        while let Some(m) = self.messages.receive()? {
            if m.id != 0 {
                if m.is_response {
                    let id = ResponseId { inner: m.id };
                    if control.waiting_for.contains(&id) {
                        control.waiting_for.remove(&id);
                        control.recieved_responses.insert(id, m);
                    }
                } else {
                    let id = RequestId { inner: m.id };
                    control.other_waiting_for.insert(id);
                    control.recieved_requests.insert(id, m);
                }
            } else {
                control.buffer.push_back(m);
            }
        }
        Ok(control
            .recieved_requests
            .pop_first()
            .map(|(i, m)| (i, m.payload)))
    }

    pub fn recieve_response(&self) -> Throws<Option<(ResponseId, T)>> {
        let mut control = self.control.lock().unwrap();
        if let Some((id, req)) = control.recieved_responses.pop_first() {
            return Ok(Some((id, req.payload)));
        }
        while let Some(m) = self.messages.receive()? {
            if m.id != 0 {
                if m.is_response {
                    let id = ResponseId { inner: m.id };
                    if control.waiting_for.contains(&id) {
                        control.waiting_for.remove(&id);
                        control.recieved_responses.insert(id, m);
                    }
                } else {
                    let id = RequestId { inner: m.id };
                    control.other_waiting_for.insert(id);
                    control.recieved_requests.insert(id, m);
                }
            } else {
                control.buffer.push_back(m);
            }
        }
        Ok(control
            .recieved_responses
            .pop_first()
            .map(|(i, m)| (i, m.payload)))
    }

    pub fn try_wait_for_response(&self, id: ResponseId) -> Throws<Option<T>> {
        let mut control = self.control.lock().unwrap();
        if let Some(m) = control.recieved_responses.remove(&id) {
            return Ok(Some(m.payload));
        }
        if !control.waiting_for.contains(&id) {
            todo!()
        }
        while let Some(m) = self.messages.receive()? {
            if m.id != 0 {
                if m.is_response {
                    let id = ResponseId { inner: m.id };
                    if control.waiting_for.contains(&id) {
                        control.waiting_for.remove(&id);
                        control.recieved_responses.insert(id, m);
                    }
                } else {
                    let id = RequestId { inner: m.id };
                    control.other_waiting_for.insert(id);
                    control.recieved_requests.insert(id, m);
                }
            } else {
                control.buffer.push_back(m);
            }
        }
        Ok(control.recieved_responses.remove(&id).map(|i| i.payload))
    }

    pub fn send(&self, value: T) -> Throws<()> {
        self.messages.send(Message {
            id: 0,
            is_response: false,
            payload: value,
        })
    }

    pub fn send_request(&self, value: T) -> Throws<ResponseId> {
        let mut ctl = self.control.lock().unwrap();
        let mut id = ResponseId { inner: 1 };
        for i in 1..=u64::MAX {
            id = ResponseId { inner: i };
            if !ctl.recieved_responses.contains_key(&id) && !ctl.waiting_for.contains(&id) {
                break;
            }
        }
        let msg = Message {
            is_response: false,
            id: id.get(),
            payload: value,
        };
        self.messages.send(msg)?;
        ctl.waiting_for.insert(id);
        Ok(id)
    }

    pub fn send_response(&self, to: RequestId, value: T) -> Throws<()> {
        let mut ctl = self.control.lock().unwrap();
        if !ctl.other_waiting_for.contains(&to) {
            todo!()
        }
        ctl.other_waiting_for.remove(&to);
        let msg = Message {
            is_response: true,
            id: to.get(),
            payload: value,
        };
        self.messages.send(msg)
    }

    pub fn send_request_wait(&self, value: T) -> Throws<T> {
        let req = self.send_request(value)?;
        loop {
            let Some(rq) = self.try_wait_for_response(req)? else {
                yield_now();
                continue;
            };
            return Ok(rq);
        }
    }

    pub fn send_request_async(&self, value: T) -> impl Future<Output = Throws<T>> {
        struct Fut<'a, T: Send + Serialize + DeserializeOwned> {
            req: ResponseId,
            slf: &'a Arachne<T>,
            err: Option<Exception>,
        }
        impl<'a, T: Send + Serialize + DeserializeOwned> Future for Fut<'a, T> {
            type Output = Throws<T>;

            fn poll(
                mut self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Self::Output> {
                if let Some(er) = self.err.take() {
                    return std::task::Poll::Ready(Err(er));
                }
                let rs = self.slf.try_wait_for_response(self.req);
                match rs {
                    Ok(x) => match x {
                        Some(out) => return std::task::Poll::Ready(Ok(out)),
                        None => {
                            return std::task::Poll::Pending;
                        }
                    },
                    Err(e) => return std::task::Poll::Ready(Err(e)),
                }
            }
        }
        let req = self.send_request(value);
        match req {
            Ok(req) => Fut {
                req,
                slf: self,
                err: None,
            },
            Err(e) => Fut {
                req: ResponseId::invalid(),
                slf: self,
                err: Some(e),
            },
        }
    }
}

pub trait ArachneId: PartialOrd + PartialEq + Ord + Eq + Copy {
    fn create(x: u64) -> Self;
    fn get(&self) -> u64;
    fn is_valid(&self) -> bool {
        self.get() != 0
    }
    fn invalid() -> Self {
        Self::create(0)
    }
}

pub fn map_store<T: ArachneId, U>(map: &mut BTreeMap<T, U>, value: U) -> T {
    let mut id;
    for i in 4096..u64::MAX {
        id = T::create(i);
        if !map.contains_key(&id) {
            map.insert(id, value);
            return id;
        }
    }
    panic!("too many keys");
}
pub fn map_store_high_priority<T: ArachneId, U>(map: &mut BTreeMap<T, U>, value: U) -> T {
    let mut id;
    for i in 1..u64::MAX {
        id = T::create(i);
        if !map.contains_key(&id) {
            map.insert(id, value);
            return id;
        }
    }
    panic!("too many keys");
}

pub fn map_remove<T: ArachneId, U>(map: &mut BTreeMap<T, U>, id: T) -> Option<U> {
    map.remove(&id)
}

pub fn map_copy<T: ArachneId, U: Clone>(map: &BTreeMap<T, U>, id: T) -> Option<U> {
    map.get(&id).map(|i| i.clone())
}

pub fn map_get<T: ArachneId, U>(map: &BTreeMap<T, U>, id: T) -> Option<&U> {
    map.get(&id)
}

pub fn map_get_mut<T: ArachneId, U>(map: &mut BTreeMap<T, U>, id: T) -> Option<&mut U> {
    map.get_mut(&id)
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PID {
    idx: u64,
}
impl ArachneId for PID {
    fn create(x: u64) -> Self {
        Self { idx: x }
    }

    fn get(&self) -> u64 {
        self.idx
    }
}

pub type SpawnProcess<T> = Box<dyn FnOnce(BPipe<LocalCmd<T>>, Option<Arachne<T>>) + Send>;
pub type SpawnProcessAsync<T> = Box<
    dyn FnOnce(BPipe<LocalCmd<T>>, Option<Arachne<T>>) -> Box<dyn Future<Output = ()> + Send>
        + Send,
>;

#[derive(Serialize, Deserialize, Clone)]
pub struct ProcessInfo {
    pub tags: Arc<[String]>,
    pub addr: SocketAddr,
    pub pid: PID,
}
pub enum LocalCmd<T: Send + Serialize + DeserializeOwned> {
    Killed,
    NewConnection(PID, Arachne<T>),
    ProcessMap(BTreeMap<PID, ProcessInfo>),
    RequestMap,
    RequestConnection(PID),
    CreateNewProcess(ProcessInfo, Arachne<T>, SpawnProcess<T>),
    CreateNewProcessAsync(ProcessInfo, Arachne<T>, SpawnProcessAsync<T>),
}

pub struct Process<T: Send + Serialize + DeserializeOwned> {
    pub info: ProcessInfo,
    pub connection: BPipe<LocalCmd<T>>,
    pub external_connections: BTreeMap<PID, Arachne<T>>,
}
#[derive(Serialize, Deserialize, Clone)]
pub enum Command<T: Send + Clone> {
    EstablishConnection {
        to: PID,
        from: PID,
    },
    CloseConnection {
        to: PID,
        from: PID,
    },
    AddressDying,
    ServerDying,
    Letter {
        to: PID,
        from: PID,
        contents: Message<T>,
    },
    NewProcess(ProcessInfo),
    ProcessDied(PID),
    AllProcesses(BTreeMap<PID, ProcessInfo>),
    RequestAllProcessees,
    GetProcessId(ProcessInfo),
    ReturnProcessId(PID),
}

pub struct CentralCommittee<T: Send + Serialize + DeserializeOwned + Clone> {
    pub connections: BTreeMap<SocketAddr, Arachne<Command<T>>>,
    pub processes: BTreeMap<PID, ProcessInfo>,
    pub listener: TcpListener,
    pub should_die: bool,
}

pub struct LocalCommittee<T: Send + Serialize + DeserializeOwned + Clone> {
    pub local_processes: BTreeMap<PID, Process<T>>,
    pub connection: Arachne<Command<T>>,
    pub listener: TcpListener,
    pub should_die: bool,
}

impl<T: Send + Serialize + DeserializeOwned + Clone> CentralCommittee<T> {
    pub fn run(&mut self) {
        while !self.should_die {
            self.update();
        }
    }

    pub fn update(&mut self) {
        self.run_listener();
        self.handle();
    }

    pub fn run_listener(&mut self) {
        while let Ok((stream, addr)) = self.listener.accept() {
            let con = Arachne::from_stream(stream);
            self.connections.insert(addr, con);
        }
    }

    pub fn handle(&mut self) {
        let mut to_forward_to_all = Vec::new();
        let mut to_forward = Vec::new();
        for (_, cons) in &mut self.connections {
            while let Ok(Some(msg)) = cons.recieve() {
                match &msg {
                    Command::EstablishConnection { to, from: _ } => to_forward.push((*to, msg)),
                    Command::CloseConnection { to, from: _ } => {
                        to_forward.push((*to, msg));
                    }
                    Command::AddressDying => {
                        to_forward_to_all.push(msg);
                    }
                    Command::ServerDying => {
                        to_forward_to_all.push(msg);
                        self.should_die = true;
                    }
                    Command::Letter {
                        to,
                        from: _,
                        contents: _,
                    } => {
                        to_forward.push((*to, msg));
                    }
                    Command::NewProcess(info) => {
                        self.processes.insert(info.pid, info.clone());
                    }
                    Command::ProcessDied(id) => {
                        self.processes.remove(id);
                        to_forward_to_all.push(msg);
                    }
                    Command::AllProcesses(_) => {}
                    Command::GetProcessId(_) => {}
                    Command::RequestAllProcessees => {}
                    Command::ReturnProcessId(_) => {}
                }
            }
            while let Ok(Some((id, msg))) = cons.recieve_request() {
                let old_id = id;
                match msg {
                    Command::GetProcessId(info) => {
                        let id = map_store(&mut self.processes, info);
                        map_get_mut(&mut self.processes, id).unwrap().pid = id;
                        cons.send_response(old_id, Command::ReturnProcessId(id))
                            .unwrap();
                    }
                    Command::RequestAllProcessees => {
                        let out = Command::AllProcesses(self.processes.clone());
                        cons.send_response(old_id, out).unwrap();
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }
        for (pid, cmd) in to_forward {
            let info = self.processes[&pid].clone();
            let sockd = info.addr;
            let pipe = &self.connections[&sockd];
            pipe.send(cmd).unwrap();
        }
        for i in to_forward_to_all {
            for (_, c) in &self.connections {
                c.send(i.clone()).unwrap();
            }
        }
    }
}

impl<T: Send + Serialize + DeserializeOwned + Clone + 'static> LocalCommittee<T> {
    pub fn update(&mut self) {
        while !self.should_die {
            self.handle();
        }
    }
    pub fn handle(&mut self) {
        for (pid, proc) in &mut self.local_processes {
            for (opid, con) in &mut proc.external_connections {
                while let Ok(Some(msg)) = con.recieve() {
                    self.connection
                        .send(Command::Letter {
                            to: *opid,
                            from: *pid,
                            contents: Message {
                                is_response: false,
                                id: 0,
                                payload: msg,
                            },
                        })
                        .unwrap();
                }
                while let Ok(Some((id, msg))) = con.recieve_request() {
                    self.connection
                        .send(Command::Letter {
                            to: *opid,
                            from: *pid,
                            contents: Message {
                                is_response: false,
                                id: id.get(),
                                payload: msg,
                            },
                        })
                        .unwrap();
                }
                while let Ok(Some((id, msg))) = con.recieve_response() {
                    self.connection
                        .send(Command::Letter {
                            to: *opid,
                            from: *pid,
                            contents: Message {
                                is_response: true,
                                id: id.get(),
                                payload: msg,
                            },
                        })
                        .unwrap();
                }
            }
        }
        while let Ok(Some(msg)) = self.connection.recieve() {
            match msg {
                Command::Letter { to, from, contents } => {
                    self.local_processes[&to].external_connections[&from]
                        .messages
                        .send(contents)
                        .unwrap();
                }
                Command::EstablishConnection { to, from } => {
                    let (con1, con2) = Arachne::new();
                    self.local_processes
                        .get_mut(&to)
                        .unwrap()
                        .external_connections
                        .insert(from, con2);
                    self.local_processes
                        .get_mut(&to)
                        .unwrap()
                        .connection
                        .send(LocalCmd::NewConnection(from, con1))
                        .unwrap();
                }
                Command::CloseConnection { to, from } => {
                    self.local_processes
                        .get_mut(&to)
                        .unwrap()
                        .external_connections
                        .remove(&from)
                        .unwrap();
                }
                Command::ServerDying => {
                    self.should_die = true;
                }
                Command::NewProcess(_) => {}
                Command::ProcessDied(pid) => {
                    for (_, p) in &mut self.local_processes {
                        p.external_connections.remove(&pid);
                    }
                }
                _ => {}
            }
        }
        let mut to_remove = Vec::new();
        let mut to_create = Vec::new();
        for (id, proc) in &mut self.local_processes {
            while let Ok(Some(rec)) = proc.connection.recieve() {
                match rec {
                    LocalCmd::Killed => {
                        to_remove.push(*id);
                    }
                    LocalCmd::NewConnection(pid, arachne) => {
                        proc.external_connections.insert(pid, arachne);
                    }
                    LocalCmd::ProcessMap(_btree_map) => {}
                    LocalCmd::RequestMap => {
                        let map = self
                            .connection
                            .send_request_wait(Command::RequestAllProcessees)
                            .unwrap();
                        match map {
                            Command::AllProcesses(map) => {
                                proc.connection.send(LocalCmd::ProcessMap(map)).unwrap();
                            }
                            _ => {}
                        }
                    }
                    LocalCmd::RequestConnection(pid) => {
                        let (p1, p2) = Arachne::new();
                        proc.external_connections.insert(pid, p1);
                        proc.connection
                            .send(LocalCmd::NewConnection(pid, p2))
                            .unwrap();
                        self.connection
                            .send(Command::EstablishConnection {
                                to: pid,
                                from: proc.info.pid,
                            })
                            .unwrap();
                    }
                    LocalCmd::CreateNewProcess(info, parent, to_call) => {
                        let mut ifx = info.clone();
                        let id = self
                            .connection
                            .send_request_wait(Command::GetProcessId(info))
                            .unwrap();
                        match id {
                            Command::ReturnProcessId(id) => {
                                let (p1, p2) = BPipe::create();
                                ifx.pid = id;
                                let proc: Process<T> = Process {
                                    info: ifx,
                                    connection: p1,
                                    external_connections: BTreeMap::new(),
                                };
                                std::thread::spawn(move || {
                                    to_call(p2, Some(parent));
                                });
                                to_create.push(proc);
                            }
                            _ => {
                                todo!()
                            }
                        }
                    }
                    LocalCmd::CreateNewProcessAsync(info, pipe, func) => {
                        let mut ifx = info.clone();
                        let id = self
                            .connection
                            .send_request_wait(Command::GetProcessId(info))
                            .unwrap();
                        match id {
                            Command::ReturnProcessId(id) => {
                                let (p1, p2) = BPipe::create();
                                ifx.pid = id;
                                let proc: Process<T> = Process {
                                    info: ifx,
                                    connection: p1,
                                    external_connections: BTreeMap::new(),
                                };
                                tokio::spawn(async move { func(p2, Some(pipe)) });
                                to_create.push(proc);
                            }
                            _ => {
                                todo!()
                            }
                        }
                    }
                }
            }
        }
        for i in to_create {
            self.local_processes.insert(i.info.pid, i);
        }
        for i in to_remove {
            self.local_processes
                .remove(&i)
                .map(|i| i.connection.send(LocalCmd::Killed));
        }
    }
}
