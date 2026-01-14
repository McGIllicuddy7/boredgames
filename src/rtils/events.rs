use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use crate::msg::{Message, Object, ObjectId};
use serde::{Serialize,Deserialize};
use async_trait::async_trait;

#[allow(unused)]
use crate::{Exception, Throw, Throws, server::HTTPRequest, server::HTTPResponse};

#[macro_export]
macro_rules! DEFINE_ID_WRAPPER {
    ($name:ident) => {
        #[allow(unused)]
        #[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize, Deserialize)]
        pub struct $name {
            inner: u64,
        }
        impl $name {
            pub fn invalid() -> Self {
                Self { inner: 0 }
            }
            pub fn inner(&self) -> u64 {
                self.inner
            }
            pub fn alloc() -> Self {
                Self {
                    inner: IDS.alloc(),
                }
            }
            pub fn alloc_high_priority() -> Self {
                Self {
                    inner: IDS.alloc_high_priority(),
                }
            }
            pub fn free(self) {
                IDS.free(self.inner)
            }
        }
    };
}
#[allow(unused)]
pub type KeyCode = char;

#[allow(unused)]
#[derive(Clone, Copy)]
pub enum MouseInput {
    LeftClick {
        x: i32,
        y: i32,
        is_currently_down: bool,
    },
    RightClick {
        x: i32,
        y: i32,
        is_currently_down: bool,
    },
    MiddleClick {
        x: i32,
        y: i32,
        is_currently_down: bool,
    },
    Move {
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        is_left_down: bool,
        is_right_down: bool,
        is_middle_down: bool,
    },
}

#[allow(unused)]
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct SubId {
    inner: u64,
}
impl SubId {
    pub fn invalid() -> Self {
        Self { inner: 0 }
    }
    pub fn is_valid(&self) -> bool {
        self.inner != 0
    }
    pub fn inner(&self) -> u64 {
        self.inner
    }
}

#[allow(unused)]
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct ServiceId {
    inner: u64,
}
impl ServiceId {
    pub fn invalid() -> Self {
        Self { inner: 0 }
    }
    pub fn is_valid(&self) -> bool {
        self.inner != 0
    }
    pub fn inner(&self) -> u64 {
        self.inner
    }
}
DEFINE_ID_WRAPPER!(TcpConnectionId);
DEFINE_ID_WRAPPER!(DaemonId);
pub trait ThreadSafeIsh: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> ThreadSafeIsh for T {}

pub trait ThreadSafeIshErr: Error + Send + Sync + 'static {}
impl<T: Error + Send + Sync + 'static> ThreadSafeIshErr for T {}
#[derive(PartialEq, Clone, Hash, Debug)]
pub enum EventType {
    CreateSubscriber,
    DestroySubscriber,
    CreateService,
    DestroyService,
    KeyBoardInput,
    MouseInput,
    TcpConnection,
    NotifyNewTcpConnection,
    TcpDisconnect,
    NetInput,
    HttpRequest,
    HttpResponse,
    NetOutput,
    UserDefined,
    CreateDaemon,
    RequestConnectionKill,
    RequestDaemonKill,
    RequestServiceKill,
    RequestKill,
    RequestSubscriberKill,
    DaemonCreated,
    Message,
    AllocObject, 
    FreeObject,
}

pub enum Event<T: ThreadSafeIsh> {
    CreateSubscriber(Box<dyn EventSub<T>>, bool),
    DestroySubscriber {
        id: SubId,
    },
    CreateService(Box<dyn Service<T>>, bool),
    DestroyService {
        id: ServiceId,
    },
    KeyBoardInput {
        key_code: KeyCode,
    },
    MouseInput {
        input_mouse_input: MouseInput,
    },
    TcpConnection {
        stream: TcpStream,
        id: TcpConnectionId,
    },
    NotifyNewTcpConnection {
        id: TcpConnectionId,
    },
    TcpDisconnect {
        id: TcpConnectionId,
    },
    NetInput {
        id: TcpConnectionId,
        data: Arc<[u8]>,
    },
    HttpRequest {
        id: TcpConnectionId,
        request: HTTPRequest,
    },
    HttpResponse {
        id: TcpConnectionId,
        response: HTTPResponse,
    },
    NetOutput {
        id: TcpConnectionId,
        data: Arc<[u8]>,
    },
    UserDefined(T),
    CreateDaemon {
        daemon: Box<dyn Daemon>,
        id: DaemonId,
    },
    DaemonCreated {
        id: DaemonId,
    },
    RequestDaemonKill {
        id: DaemonId,
    },
    RequestConnectionKill {
        id: TcpConnectionId,
    },
    RequestSubscriberKill {
        id: SubId,
    },
    RequestServiceKill {
        id: ServiceId,
    },
    RequestKill {
        id: u64,
    },
    Message(Message),
    AllocObject{
        id:ObjectId,
        object:Box<dyn Object>
    }, 
    FreeObject{
            id:ObjectId,
    },
}
impl<T: Clone + ThreadSafeIsh> Event<T> {
    pub fn try_clone(&self) -> Option<Event<T>> {
        match self {
            Event::CreateSubscriber(_, _) => None,
            Event::DestroySubscriber { id: _ } => None,
            Event::CreateService(_,_) => None,
            Event::DestroyService { id: _ } => None,
            Event::KeyBoardInput { key_code } => Some(Event::KeyBoardInput {
                key_code: *key_code,
            }),
            Event::MouseInput { input_mouse_input } => Some(Event::MouseInput {
                input_mouse_input: *input_mouse_input,
            }),
            Event::TcpConnection { stream: _, id: _ } => None,
            Event::NotifyNewTcpConnection { id } => Some(Event::NotifyNewTcpConnection { id: *id }),
            Event::TcpDisconnect { id } => Some(Event::TcpDisconnect { id: *id }),
            Event::HttpRequest { id, request } => Some(Event::HttpRequest {
                id: *id,
                request: request.clone(),
            }),
            Event::HttpResponse { id, response } => Some(Event::HttpResponse {
                id: *id,
                response: response.clone(),
            }),
            Event::NetOutput { id, data } => Some(Event::NetOutput {
                id: *id,
                data: data.clone(),
            }),
            Event::NetInput { id, data } => Some(Event::NetInput {
                id: *id,
                data: data.clone(),
            }),
            Event::UserDefined(t) => Some(Event::UserDefined(t.clone())),
            Event::CreateDaemon { daemon: _, id: _ } => None,
            Event::RequestKill { id } => Some(Event::RequestKill { id: *id }),
            Event::RequestConnectionKill { id } => Some(Event::RequestConnectionKill { id: *id }),
            Event::RequestServiceKill { id } => Some(Event::RequestServiceKill { id: *id }),
            Event::RequestSubscriberKill { id } => Some(Event::RequestSubscriberKill { id: *id }),
            Event::RequestDaemonKill { id } => Some(Event::RequestDaemonKill { id: *id }),
            Event::DaemonCreated { id } => Some(Event::DaemonCreated { id: *id }),
            Event::Message(message)=>Some(Event::Message(message.clone())),
            Event::AllocObject { id:_, object:_, }=>None, 
            Event::FreeObject { id:_ }=>None,
        }
    }

    pub fn is_clonable(&self) -> bool {
        match self {
            Event::CreateSubscriber(_,_) => false,
            Event::DestroySubscriber { id: _ } => false,
            Event::CreateService(_,_) => false,
            Event::DestroyService { id: _ } => false,
            Event::KeyBoardInput { key_code: _ } => true,
            Event::MouseInput {
                input_mouse_input: _,
            } => true,
            Event::TcpConnection { stream: _, id: _ } => false,
            Event::UserDefined(_) => true,
            Event::CreateDaemon { daemon: _, id: _ } => false,
            Event::NotifyNewTcpConnection { id: _ } => true,
            Event::TcpDisconnect { id: _ } => true,
            Event::NetInput { id: _, data: _ } => true,
            Event::NetOutput { id: _, data: _ } => true,
            Event::HttpRequest { id: _, request: _ } => true,
            Event::HttpResponse { id: _, response: _ } => true,
            Event::RequestConnectionKill { id: _ } => true,
            Event::RequestKill { id: _ } => true,
            Event::RequestDaemonKill { id: _ } => true,
            Event::RequestSubscriberKill { id: _ } => true,
            Event::RequestServiceKill { id: _ } => true,
            Event::DaemonCreated { id:_ } => true,
            Event::Message(_)=>true,
            Event::AllocObject { id:_, object:_ }=>false, 
            Event::FreeObject { id:_ }=>false,
        }
    }
}
impl<T:ThreadSafeIsh> Event<T>{
        pub fn get_type(&self) -> EventType {
        match self {
            Event::CreateSubscriber(_,_) => EventType::CreateSubscriber,
            Event::DestroySubscriber { id: _ } => EventType::DestroySubscriber,
            Event::CreateService(_,_) => EventType::CreateService,
            Event::DestroyService { id: _ } => EventType::DestroyService,
            Event::KeyBoardInput { key_code: _ } => EventType::KeyBoardInput,
            Event::MouseInput {
                input_mouse_input: _,
            } => EventType::MouseInput,
            Event::TcpConnection { stream: _, id: _ } => EventType::TcpConnection,
            Event::UserDefined(_) => EventType::UserDefined,
            Event::CreateDaemon { daemon: _, id: _ } => EventType::CreateDaemon,
            Event::NotifyNewTcpConnection { id: _ } => EventType::NotifyNewTcpConnection,
            Event::TcpDisconnect { id: _ } => EventType::TcpDisconnect,
            Event::NetInput { id: _, data: _ } => EventType::NetInput,
            Event::NetOutput { id: _, data: _ } => EventType::NetOutput,
            Event::HttpRequest { id: _, request: _ } => EventType::HttpRequest,
            Event::HttpResponse { id: _, response: _ } => EventType::HttpResponse,
            Event::RequestConnectionKill { id: _ } => EventType::RequestConnectionKill,
            Event::RequestDaemonKill { id: _ } => EventType::RequestDaemonKill,
            Event::RequestServiceKill { id: _ } => EventType::RequestServiceKill,
            Event::RequestSubscriberKill { id: _ } => EventType::RequestSubscriberKill,
            Event::DaemonCreated { id: _ } => EventType::DaemonCreated,
            Event::RequestKill { id: _ } => EventType::RequestKill,
            Event::Message(_)=>EventType::Message,
            Event::AllocObject { id:_, object:_ }=>EventType::AllocObject, 
            Event::FreeObject { id:_,}=>EventType::FreeObject,
        }
    }
}

pub enum EventRequest {
    None,
    Shared,
    Owned,
}

#[async_trait]
pub trait EventSub<T: ThreadSafeIsh>: ThreadSafeIsh {
    async fn on_create(&mut self, self_id: SubId, sender: EventSync<T>);
    async fn wants_event(&self, event: &T) -> Throws<EventRequest> {
        _ = event;
        Ok(EventRequest::None)
    }

    async fn wants_global_event(&self, event: &Event<T>) -> Throws<EventRequest> {
        _ = event;
        Ok(EventRequest::None)
    }

    async fn on_event_owned(&mut self, event: T) -> Throws<()> {
        _ = event;
        Ok(())
    }

    async fn on_event(&mut self, event: &T) -> Throws<()>;

    async fn on_global_event<'a>(&'a self, event: &Event<T>) -> Throws<()> {
        _ = event;
        Ok(())
    }

    async fn on_global_event_owned(&mut self, event: Event<T>) -> Throws<()> {
        _ = event;
        Ok(())
    }
}

#[async_trait]
pub trait Service<T: ThreadSafeIsh>: ThreadSafeIsh {
    async fn create(&mut self, id: ServiceId, sender: EventSync<T>);
    async fn update(&mut self) -> Throws<()>;
}

#[async_trait]
pub trait Daemon: ThreadSafeIsh {
    async fn run(&mut self);
}
struct Handler<T: ThreadSafeIsh> {
    subscribers: BTreeMap<SubId, Box<dyn EventSub<T>>>,
    services: BTreeMap<ServiceId, Box<dyn Service<T>>>,
    daemons: BTreeSet<DaemonId>,
    objects:BTreeMap<ObjectId, Box<dyn Object>>,
    sender: Sender<Event<T>>,
}

pub struct EventHandler<T: ThreadSafeIsh> {
    pub channel: Receiver<Event<T>>,
    handler: Handler<T>,
}

impl<T: ThreadSafeIsh> Handler<T> {
    fn new(sender: Sender<Event<T>>) -> Self {
        Self {
            subscribers: BTreeMap::new(),
            sender,
            services: BTreeMap::new(),
            daemons: BTreeSet::new(),
            objects:BTreeMap::new(),
        }
    }

    pub async fn create_subscriber(
        &mut self,
        mut event_sub: Box<dyn EventSub<T>>,
        sender: Sender<Event<T>>,
    ) {
        let mut min = 2048;
        for i in 2048..=u64::MAX {
            if !self.subscribers.contains_key(&SubId { inner: i }) {
                min = i;
                break;
            }
        }
        if min == u64::MAX {
            panic!();
        }
        event_sub
            .as_mut()
            .on_create(SubId { inner: min }, EventSync::new(sender))
            .await;
        self.subscribers.insert(SubId { inner: min }, event_sub);
    }
    pub async fn create_subscriber_high_priority(
        &mut self,
        mut event_sub: Box<dyn EventSub<T>>,
        sender: Sender<Event<T>>,
    ) {
        let mut min = 0;
        for i in 0..=u64::MAX {
            if !self.subscribers.contains_key(&SubId { inner: i }) {
                min = i;
                break;
            }
        }
        if min == u64::MAX {
            panic!();
        }
        event_sub
            .as_mut()
            .on_create(SubId { inner: min }, EventSync::new(sender))
            .await;
        self.subscribers.insert(SubId { inner: min }, event_sub);
    }

    pub async fn destroy_subscriber(&mut self, id: SubId) {
        self.subscribers.remove(&id);
    }

    pub async fn create_service(
        &mut self,
        mut service: Box<dyn Service<T>>,
        sender: Sender<Event<T>>,
    ) {
        let mut min = 2048;
        for i in 2048..=u64::MAX {
            if !self.services.contains_key(&ServiceId { inner: min }) {
                min = i;
                break;
            }
        }
        if min == u64::MAX {
            panic!();
        }
        service
            .create(ServiceId { inner: min }, EventSync::new(sender))
            .await;
        self.services.insert(ServiceId { inner: min }, service);
    }

    pub async fn create_service_high_priority(
        &mut self,
        mut service: Box<dyn Service<T>>,
        sender: Sender<Event<T>>,
    ) {
        let mut min = 0;
        for i in 0..=u64::MAX {
            if !self.services.contains_key(&ServiceId { inner: min }) {
                min = i;
                break;
            }
        }
        if min == u64::MAX {
            panic!();
        }
        service
            .create(ServiceId { inner: min }, EventSync::new(sender))
            .await;
        self.services.insert(ServiceId { inner: min }, service);
    }

    pub async fn destroy_service(&mut self, id: ServiceId) {
        self.services.remove(&id);
    }

    pub async fn handle_user_event(&mut self, ev: T) -> Throws<()> {
        for i in &mut self.subscribers {
            let v = i.1.wants_event(&ev).await?;
            match v {
                EventRequest::None => {
                    continue;
                }
                EventRequest::Shared => {
                    i.1.as_mut().on_event(&ev).await?;
                }
                EventRequest::Owned => {
                    i.1.as_mut().on_event_owned(ev).await?;
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn run_event(&mut self, i: Event<T>) -> Throws<()> {
        for (_, sub) in &mut self.subscribers {
            match sub.as_ref().wants_global_event(&i).await? {
                EventRequest::None => {
                    continue;
                }
                EventRequest::Shared => {
                    sub.as_mut().on_global_event(&i).await?;
                }
                EventRequest::Owned => {
                    sub.as_mut().on_global_event_owned(i).await?;
                    return Ok(());
                }
            }
        }
        println!("LOG:{:#?}", i.get_type());
        match i {
            Event::CreateSubscriber(event_sub,high_priority) => {
                if high_priority{
                    self.create_subscriber_high_priority(event_sub, self.sender.clone()).await;
                }else{
                    self.create_subscriber(event_sub, self.sender.clone()).await;
                }

            }
            Event::DestroySubscriber { id } => {
                self.destroy_subscriber(id).await;
            }
            Event::CreateService(service,high_priority) => {
                if high_priority{
                    self.create_service_high_priority(service, self.sender.clone()).await; 
                }else{
                    self.create_service(service, self.sender.clone()).await;
                }
            }
            Event::DestroyService { id } => {
                self.destroy_service(id).await;
            }
            Event::UserDefined(x) => {
                self.handle_user_event(x).await?;
            }
            Event::CreateDaemon { mut daemon, id } => {
                tokio::task::spawn(async move { daemon.run().await });
                self.daemons.insert(id);
                self.sender.send(Event::DaemonCreated { id })?;
            }
            Event::RequestDaemonKill { id } => {
                self.daemons.remove(&id);
                id.free();
            }
            Event::RequestServiceKill { id } => {
                self.services.remove(&id);
            }
            Event::RequestSubscriberKill { id } => {
                self.subscribers.remove(&id);
            }
            Event::KeyBoardInput { key_code:_ } => {
                todo!();
            },
            Event::MouseInput { input_mouse_input:_ } =>{
                todo!();
            },
            Event::Message(msg)=>{
                let id = msg.target_id;
                if let Some(obj) = self.objects.get_mut(&id){
                    obj.as_mut().call(msg);
                }
            }
            Event::AllocObject { id, object }=>{
                self.objects.insert(id, object);
            }
            Event::FreeObject { id }=>{
                self.objects.remove(&id);
                id.free();
            }
            _=>{

            }
        }
        Ok(())
    }
}

impl<T: ThreadSafeIsh> EventHandler<T> {
    pub fn new() -> (std::sync::mpsc::Sender<Event<T>>, Self) {
        let (sender, reciever) = std::sync::mpsc::channel();
        (
            sender.clone(),
            Self {
                channel: reciever,
                handler: Handler::new(sender),
            },
        )
    }

    pub async fn handle_events(&mut self) -> Throws<()> {
        loop {
            let ev = self.channel.try_recv();
            match ev {
                Ok(ev) => {
                    self.handler.run_event(ev).await?;
                }
                Err(e) => match e {
                    std::sync::mpsc::TryRecvError::Empty => {
                        break;
                    }
                    std::sync::mpsc::TryRecvError::Disconnected => {
                        return Err("disconnected".into());
                    }
                },
            }
        }
        Ok(())
    }

    pub async fn handle_services(&mut self) -> Throws<()> {
        for service in self.handler.services.values_mut() {
            service.as_mut().update().await?;
        }
        Ok(())
    }

    pub async fn run(&mut self, setup: impl AsyncFn(EventSync<T>)) {
        setup(EventSync::new(self.handler.sender.clone())).await;
        loop {
            let res = self.handle_events().await;
            if res.is_err() {
                todo!();
            }
            let res = self.handle_services().await;
            if res.is_err() {
                todo!()
            }
        }
    }
}

pub struct EventSync<T: ThreadSafeIsh> {
    sender: Option<Sender<Event<T>>>,
}
impl<T: ThreadSafeIsh> Clone for EventSync<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}
impl<T: ThreadSafeIsh> EventSync<T> {
    pub fn new(sender: Sender<Event<T>>) -> Self {
        Self {
            sender: Some(sender),
        }
    }

    pub fn new_event(&self, ev: T) -> Throws<()> {
        self.sender
            .as_ref()
            .unwrap()
            .send(Event::UserDefined(ev))
            .unwrap();
        Ok(())
    }

    pub fn new_event_global(&self, ev: Event<T>) -> Throws<()> {
        self.sender.as_ref().unwrap().send(ev).unwrap();
        Ok(())
    }

    pub fn create_new_subscriber<Sub: EventSub<T> + 'static>(
        &self,
        subscriber: Sub,
    ) -> Result<(), Box<dyn Error>> {
        let bx = Box::new(subscriber) as Box<dyn EventSub<T>>;
        self.sender
            .as_ref()
            .unwrap()
            .send(Event::CreateSubscriber(bx,false))?;
        Ok(())
    }

    pub fn create_new_service<Serv:Service<T>+ 'static>(
        &self,
        service:Serv
    ) -> Result<(), Box<dyn Error>> {
        let bx = Box::new(service) as Box<dyn Service<T>>;
        self.sender
            .as_ref()
            .unwrap()
            .send(Event::CreateService(bx,false))?;
        Ok(())
    }

    pub fn create_new_service_high_priority<Serv:Service<T>+ 'static>(
        &self,
        service:Serv
    ) -> Result<(), Box<dyn Error>> {
        let bx = Box::new(service) as Box<dyn Service<T>>;
        self.sender
            .as_ref()
            .unwrap()
            .send(Event::CreateService(bx,true))?;
        Ok(())
    }

    pub fn destroy_subscriber(&self, id:SubId)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::DestroySubscriber { id })?;
        Ok(())
    }

    pub fn destroy_service(&self, id:ServiceId)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::DestroyService { id })?;
        Ok(())
    } 

    pub fn tcp_connection(&self, stream:TcpStream, id:TcpConnectionId)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::TcpConnection {stream, id })?;
        Ok(())
    }

    pub fn new_tcp_connection(&self, id:TcpConnectionId)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::NotifyNewTcpConnection { id })?;
        Ok(())
    }

    pub fn tcp_disconnect(&self, id:TcpConnectionId)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::NotifyNewTcpConnection { id })?;
        Ok(())
    }

    pub fn net_input(&self, id:TcpConnectionId, data:Arc<[u8]>)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::NetInput { id, data })?;
        Ok(())
    }

    pub fn http_request(&self, id:TcpConnectionId, request:HTTPRequest)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::HttpRequest { id, request })?;
        Ok(())
    }

    pub fn http_response(&self, id:TcpConnectionId, response:HTTPResponse)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::HttpResponse { id, response })?;
        Ok(())
    }

    pub fn net_output(&self,id:TcpConnectionId, data:Arc<[u8]>)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::NetOutput { id, data})?;
        Ok(()) 
    }
    pub fn create_daemon(&self, daemon:Box<dyn Daemon>, id:DaemonId)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::CreateDaemon { daemon, id })?;
        Ok(())
    }

    pub fn kill_daemon(&self, to_kill:DaemonId)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::RequestDaemonKill { id: to_kill})?;
        Ok(()) 
    }

    pub fn kill_connection(&self, to_kill:TcpConnectionId)->Throws<()>{
           self.sender.as_ref().unwrap().send(Event::RequestConnectionKill { id: to_kill})?;
        Ok(())  
    }

    pub fn kill_service(&self, to_kill:ServiceId)->Throws<()>{
           self.sender.as_ref().unwrap().send(Event::RequestServiceKill { id: to_kill})?;
        Ok(())  
    }
        
    pub fn kill_subscriber(&self, to_kill:SubId)->Throws<()>{
           self.sender.as_ref().unwrap().send(Event::RequestSubscriberKill { id: to_kill})?;
        Ok(())  
    }

    pub fn new_message(&self, msg:Message)->Throws<()>{
        self.sender.as_ref().unwrap().send(Event::Message(msg))?;
        Ok(())   
    }

    pub fn invalid() -> Self {
        Self { sender: None }
    }
}

#[derive(Clone)]
pub struct BPipe<T> {
    sending: Arc<Mutex<VecDeque<T>>>,
    recieving: Arc<Mutex<VecDeque<T>>>,
    done: Arc<AtomicBool>,
}
impl<T> BPipe<T> {
    pub fn create() -> (BPipe<T>, BPipe<T>) {
        let p1 = Arc::new(Mutex::new(VecDeque::new()));
        let p2 = Arc::new(Mutex::new(VecDeque::new()));
        let done = Arc::new(AtomicBool::new(false));
        let out1 = BPipe {
            sending: p1.clone(),
            recieving: p2.clone(),
            done: done.clone(),
        };
        let out2 = BPipe {
            sending: p2,
            recieving: p1,
            done,
        };
        (out1, out2)
    }

    pub fn send(&self, v: T) -> Throws<()> {
        if self.done.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("done".into());
        }
        let mut sending = self.sending.lock().unwrap();
        sending.push_back(v);
        Ok(())
    }

    pub fn recieve(&self) -> Throws<Option<T>> {
        if self.done.load(std::sync::atomic::Ordering::Relaxed) {
            todo!()
        }
        let mut recieving = self.recieving.lock().unwrap();
        Ok(recieving.pop_front())
    }

    pub fn recieve_wait(&self) -> Throws<T> {
        loop {
            if self.done.load(std::sync::atomic::Ordering::Relaxed) {
                return Err("done".into());
            }
            let mut recieving = self.recieving.lock().unwrap();
            if let Some(x) = recieving.pop_front() {
                return Ok(x);
            }
        }
    }

    pub fn recieve_async(&self) -> impl Future<Output = Throws<T>> {
        pub struct Out<T> {
            reciever: Arc<Mutex<VecDeque<T>>>,
            done: Arc<AtomicBool>,
        }
        impl<T> Future for Out<T> {
            type Output = Throws<T>;
            fn poll(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Self::Output> {
                if self.done.load(std::sync::atomic::Ordering::Relaxed) {
                    return std::task::Poll::Ready(Err::<T, Exception>("done".into()));
                }
                let tmp = self.reciever.try_lock();
                match tmp {
                    Ok(mut x) => {
                        if let Some(x) = x.pop_front() {
                            std::task::Poll::Ready(Ok(x))
                        } else {
                            std::task::Poll::Pending
                        }
                    }
                    Err(er) => match er {
                        std::sync::TryLockError::Poisoned(poison_error) => {
                            let mut e = poison_error.into_inner();
                            if let Some(x) = e.pop_front() {
                                std::task::Poll::Ready(Ok(x))
                            } else {
                                std::task::Poll::Pending
                            }
                        }
                        std::sync::TryLockError::WouldBlock => std::task::Poll::Pending,
                    },
                }
            }
        }
        Out {
            reciever: self.recieving.clone(),
            done: self.done.clone(),
        }
    }
}
impl<T> Drop for BPipe<T> {
    fn drop(&mut self) {
        self.done.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

pub struct EventForwarder<T: ThreadSafeIsh> {
    self_id: SubId,
    sender: EventSync<T>,
    pipe: Option<BPipe<T>>,
    should_forward: Box<dyn Fn(&T) -> bool + Send + Sync + 'static>,
    event_pipe: Option<BPipe<Event<T>>>,
    should_forward_event: Box<dyn Fn(&Event<T>) -> bool + Send + Sync + 'static>,
}
impl<T: ThreadSafeIsh + Clone> EventForwarder<T> {
    pub async fn new(
        should_forward: impl Fn(&T) -> bool + Send + Sync + 'static,
        sync: EventSync<T>,
    ) -> BPipe<T> {
        let (this_pipe, out_pipe) = BPipe::create();
        fn f_false<T: ThreadSafeIsh>(_: &Event<T>) -> bool {
            false
        }
        let this = Self {
            pipe: Some(this_pipe),
            should_forward: Box::new(should_forward),
            event_pipe: None,
            should_forward_event: Box::new(f_false),
            self_id: SubId::invalid(),
            sender: EventSync::invalid(),
        };
        sync.create_new_subscriber(this).unwrap();
        out_pipe
    }

    pub async fn new_globals(
        should_forward: impl Fn(&Event<T>) -> bool + Send + Sync + 'static,
        sync: EventSync<T>,
    ) -> BPipe<Event<T>> {
        let (this_pipe, out_pipe) = BPipe::create();
        fn f_false<T: ThreadSafeIsh>(_: &T) -> bool {
            false
        }
        let this = Self {
            pipe: None,
            should_forward: Box::new(f_false),
            event_pipe: Some(this_pipe),
            should_forward_event: Box::new(should_forward),
            self_id: SubId::invalid(),
            sender: EventSync::invalid(),
        };
        sync.create_new_subscriber(this).unwrap();
        out_pipe
    }

    pub async fn new_all(
        should_forward: impl Fn(&T) -> bool + Send + Sync + 'static,
        should_forward_globals: impl Fn(&Event<T>) -> bool + Send + Sync + 'static,
        sync: EventSync<T>,
    ) -> (BPipe<T>, BPipe<Event<T>>) {
        let (this_pipe, out_pipe) = BPipe::create();
        let (this_events, out_events) = BPipe::create();
        let this = Self {
            pipe: Some(this_pipe),
            should_forward: Box::new(should_forward),
            event_pipe: Some(this_events),
            should_forward_event: Box::new(should_forward_globals),
            self_id: SubId::invalid(),
            sender: EventSync::invalid(),
        };
        sync.create_new_subscriber(this).unwrap();
        (out_pipe, out_events)
    }
}

#[async_trait]
impl<T: ThreadSafeIsh + Clone> EventSub<T> for EventForwarder<T> {
    async fn on_create(&mut self, self_id: SubId, sender: EventSync<T>) {
        self.self_id = self_id;
        self.sender = sender;
    }

    async fn on_event(&mut self, event: &T) -> Throws<()> {
        self.pipe.as_ref().unwrap().send(event.clone())
    }

    async fn on_global_event<'a>(&'a self, event: &Event<T>) -> Throws<()> {
        let Some(event) = event.try_clone() else {
            return Ok(());
        };
        self.event_pipe.as_ref().unwrap().send(event)?;
        Ok(())
    }

    async fn wants_global_event(&self, event: &Event<T>) -> Throws<EventRequest> {
        if self.event_pipe.is_none() {
            return Ok(EventRequest::None);
        }
        if event.is_clonable() {
            if (self.should_forward_event)(event) {
                Ok(EventRequest::Shared)
            } else {
                Ok(EventRequest::None)
            }
        } else {
            Ok(EventRequest::None)
        }
    }

    async fn wants_event(&self, event: &T) -> Throws<EventRequest> {
        if (self.should_forward)(event) {
            Ok(EventRequest::Shared)
        } else {
            Ok(EventRequest::None)
        }
    }
}

pub struct IdAllocator {
    al: Mutex<IdAllocatorInternal>,
}
impl IdAllocator {
    pub fn alloc(&self) -> u64 {
        self.al.lock().unwrap().alloc()
    }
    pub fn alloc_high_priority(&self) -> u64 {
        self.al.lock().unwrap().alloc_high_priority()
    }
    pub fn free(&self, id: u64) {
        self.al.lock().unwrap().free(id)
    }
}

pub static IDS: IdAllocator = IdAllocator {
    al: Mutex::new(IdAllocatorInternal::new()),
};

struct IdAllocatorInternal {
    general_ids: BTreeSet<u64>,
}
impl IdAllocatorInternal {
    pub const fn new() -> Self {
        Self {
            general_ids: BTreeSet::new(),
        }
    }

    pub fn alloc(&mut self) -> u64 {
        let mut min = 2048;
        for i in 2048..=u64::MAX {
            min = i;
            if !self.general_ids.contains(&i) {
                break;
            }
        }
        if min == u64::MAX {
            panic!()
        }
        self.general_ids.insert(min);
        min
    }
    pub fn alloc_high_priority(&mut self) -> u64 {
        let mut min = 1;
        for i in 1..=u64::MAX {
            min = i;
            if !self.general_ids.contains(&i) {
                break;
            }
        }
        if min == u64::MAX {
            panic!()
        }
        self.general_ids.insert(min);
        min
    }
    pub fn free(&mut self, id: u64) {
        self.general_ids.remove(&id);
    }
}

pub struct WriteOnce<T> {
    v: Arc<Mutex<Option<T>>>,
}
impl<T> WriteOnce<T> {
    pub fn create() -> (Self, Self) {
        let vout = Arc::new(Mutex::new(None));
        let a = Self { v: vout.clone() };
        let b = Self { v: vout };
        (a, b)
    }
    pub fn read(&self) -> impl Future<Output = Result<T, Box<dyn Error>>> {
        struct Out<T> {
            v: Arc<Mutex<Option<T>>>,
        }
        impl<T> Future for Out<T> {
            type Output = Result<T, Box<dyn Error>>;
            fn poll(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Self::Output> {
                let lock = self.v.try_lock();
                match lock {
                    Ok(mut t) => {
                        if let Some(m) = t.take() {
                            std::task::Poll::Ready(Ok(m))
                        } else {
                            std::task::Poll::Pending
                        }
                    }
                    Err(e) => match e {
                        std::sync::TryLockError::Poisoned(poison_error) => {
                            let mut lck = poison_error.into_inner();
                            if let Some(m) = lck.take() {
                                std::task::Poll::Ready(Ok(m))
                            } else {
                                std::task::Poll::Pending
                            }
                        }
                        std::sync::TryLockError::WouldBlock => std::task::Poll::Pending,
                    },
                }
            }
        }
        let out = Out { v: self.v.clone() };
        out
    }

    pub fn write(self, v: T) {
        let lock = self.v.lock();
        let mut value = match lock {
            Ok(value) => value,
            Err(value) => value.into_inner(),
        };
        *value = Some(v);
    }

    pub fn try_read(&self) -> Result<Option<T>, Box<dyn Error>> {
        let lock = self.v.try_lock();
        match lock {
            Ok(mut t) => {
                if let Some(m) = t.take() {
                    Ok(Some(m))
                } else {
                    Ok(None)
                }
            }
            Err(e) => match e {
                std::sync::TryLockError::Poisoned(poison_error) => {
                    let mut lck = poison_error.into_inner();
                    if let Some(m) = lck.take() {
                        Ok(Some(m))
                    } else {
                        Ok(None)
                    }
                }
                std::sync::TryLockError::WouldBlock => Ok(None),
            },
        }
    }
}
