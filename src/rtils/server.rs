use std::{collections::HashSet, sync::Arc, time::Duration};

use async_trait::async_trait;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}};

use crate::{Exception, Throw, Throws, events::{BPipe, Daemon, DaemonId, Event, EventForwarder, EventHandler, EventSync, EventType,  TcpConnectionId, ThreadSafeIsh}, throw};
/*
* HTTP request enum, we will accept a body in any request for compatability with dumbshit because I
* am a good girl
* */

#[derive(Debug, Clone)]
pub enum HTTPRequest {
    Get { target: Arc<str>, msg: Arc<[u8]> },
    Head { target: Arc<str>, msg: Arc<[u8]> },
    Post { target: Arc<str>, msg: Arc<[u8]> },
    Put { target: Arc<str>, msg: Arc<[u8]> },
    Delete { target: Arc<str>, msg: Arc<[u8]> },
    Connect { target: Arc<str>, msg: Arc<[u8]> },
    Options { target: Arc<str>, msg: Arc<[u8]> },
    Trace { target: Arc<str>, msg: Arc<[u8]> },
    Patch { target: Arc<str>, msg: Arc<[u8]> },
}

#[derive(Debug, Clone)]
pub enum HttpResponseType {
    Text,
    Html,
    Png,
    Jpeg,
    Json,
    Js,
}
#[derive(Debug, Clone)]
pub struct HTTPResponse {
    pub response_type: HttpResponseType,
    pub data: Arc<[u8]>,
}

pub async fn http_write_string_response(stream: &mut TcpStream, s: &[u8]) -> Throws<()> {
    let f = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\n",
        s.len()
    );
    stream.write(f.as_bytes()).await?;
    stream.write(s).await?;
    Ok(())
}

pub async fn http_write_html_response(stream: &mut TcpStream, s: &[u8]) -> Throws<()> {
    let f = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\n",
        s.len()
    );
    stream.write(f.as_bytes()).await?;
    stream.write(s).await?;
    Ok(())
}

pub async fn http_write_png_response(stream: &mut TcpStream, s: &[u8]) -> Throws<()> {
    let f = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\ncharset=UTF-8\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\n",
        s.len()
    );
    stream.write(f.as_bytes()).await?;
    stream.write(s).await?;
    Ok(())
}

pub async fn http_write_jpg_response(stream: &mut TcpStream, s: &[u8]) -> Throws<()> {
    let f = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\ncharset=UTF-8\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\n",
        s.len()
    );
    stream.write(f.as_bytes()).await?;
    stream.write(s).await?;
    Ok(())
}

pub async fn http_write_json_response(stream: &mut TcpStream, s: &[u8]) -> Throws<()> {
    let f = format!(
        "HTTP/1.1 200 OK\r\nContent-Type:text/json\r\ncharset=UTF-8\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\n",
        s.len()
    );
    stream.write(f.as_bytes()).await?;
    stream.write(s).await?;
    Ok(())
}

pub async fn http_write_js_response(stream: &mut TcpStream, s: &[u8]) -> Throws<()> {
    let f = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/javascript; charset=UTF-8\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\n",
        s.len()
    );
    stream.write(f.as_bytes()).await?;
    stream.write(s).await?;
    Ok(())
}

pub fn get_extension(s: &str) -> &str {
    let split = s.split('.');
    let mut last = None;
    for i in split {
        last = Some(i);
    }
    let ext = if let Some(e) = last { e } else { "" };
    ext
}

#[test]
fn extension_tests() {
    assert_eq!(get_extension("test.png"), "png");
    assert_eq!(get_extension("test.bak.png"), "png");
    assert_eq!(get_extension("test.jpeg"), "jpeg");
    assert_eq!(get_extension("test.bak.jpeg"), "jpeg");
}

pub async fn read_line(stream: &mut TcpStream) -> Throws<Vec<u8>> {
    let mut idx = 0;
    let mut tbuf = [0];
    let mut buf = [0; 256];
    let mut out = Vec::new();
    loop {
        let x = stream.read(&mut tbuf).await?;
        if x == 0 {
            break;
        }
        if tbuf[0] == b'\n' {
            break;
        }
        buf[idx] = tbuf[0];
        idx += 1;
        if buf.len() <= idx {
            for i in buf {
                out.push(i);
            }
            idx = 0;
        }
    }
    for i in 0..idx {
        out.push(buf[i]);
    }
    return Ok(out);
}

pub async fn http_get_request(stream: &mut TcpStream) -> Throws<HTTPRequest> {
    loop{
    let header = read_line(stream).await?;
    let header_string = std::str::from_utf8(&header)?;
    let mut args = header_string.split_ascii_whitespace();
    if args.clone().count() == 0{
        continue;
    }
    let cmd = args.next().throw()?;
    match cmd {
        "GET" => {
            return http_parse_get(header_string, stream).await;
        }
        "HEAD" => {
            return http_parse_head(header_string, stream).await;
        }
        "POST" => {
            return http_parse_post(header_string, stream).await;
        }
        "PUT" => {
            return http_parse_put(header_string, stream).await;
        }
        "CONNECT" => {
            return http_parse_connect(header_string, stream).await;
        }
        "DELETE" => {
            return http_parse_delete(header_string, stream).await;
        }
        _ => {
            throw!(format!("error unknown argument to http request:{:#?}", cmd));
        }
    }
    }
}

pub async fn http_parse_get(header_string: &str, stream: &mut TcpStream) -> Throws<HTTPRequest> {
    let mut l1 = header_string.split_ascii_whitespace();
    let x = l1.next().throw()?;
    let target = l1.next().throw()?;
    assert!(x == "GET");
    let mut cl = 0;
    loop {
        let s = read_line(stream).await?;
        let s = str::from_utf8(&s)?.trim();
        let xs = s.split_once(":");
        if let Some((start, remainder)) = xs {
            if start == "Content-Length" {
                cl = remainder.trim().parse::<usize>()?;
            }
        }
        if s.len() == 0 {
            break;
        }
    }
    let mut buf = Vec::with_capacity(cl);
    for _ in 0..cl {
        buf.push(0);
    }
    _ = stream.read_exact(&mut buf).await?;
    Ok(HTTPRequest::Get {
        target: target.into(),
        msg: buf.into(),
    })
}

pub async fn http_parse_head(header_string: &str, stream: &mut TcpStream) -> Throws<HTTPRequest> {
    let mut l1 = header_string.split_ascii_whitespace();
    let x = l1.next().throw()?;
    let target = l1.next().throw()?;
    assert!(x == "HEAD");
    let mut cl = 0;
    loop {
        let s = read_line(stream).await?;
        let s = str::from_utf8(&s)?;
        let xs = s.split_once(":");
        if let Some((start, remainder)) = xs {
            if start == "Content-Length" {
                cl = remainder.trim().parse::<usize>()?;
            }
        }
        if s.len() == 0 {
            break;
        }
    }
    let mut buf = Vec::with_capacity(cl);
    for _ in 0..cl {
        buf.push(0);
    }
    _ = stream.read_exact(&mut buf).await?;
    Ok(HTTPRequest::Head {
        target: target.into(),
        msg: buf.into(),
    })
}

pub async fn http_parse_post(header_string: &str, stream: &mut TcpStream) -> Throws<HTTPRequest> {
    let mut l1 = header_string.split_ascii_whitespace();
    let x = l1.next().throw()?;
    let target = l1.next().throw()?;
    assert!(x == "POST");
    let mut cl = 0;
    loop {
        let s = read_line(stream).await?;
        let s = str::from_utf8(&s)?.trim();
        let xs = s.split_once(":");
        if let Some((start, remainder)) = xs {
            if start == "Content-Length" {
                cl = remainder.trim().parse::<usize>()?;
            }
        }
        if s.len() == 0 {
            break;
        }
    }
    let mut buf = Vec::with_capacity(cl);
    for _ in 0..cl {
        buf.push(0);
    }
    _ = stream.read_exact(&mut buf).await.unwrap();
    Ok(HTTPRequest::Post {
        target: target.into(),
        msg: buf.into(),
    })
}

pub async fn http_parse_put(header_string: &str, stream: &mut TcpStream) -> Throws<HTTPRequest> {
    let mut l1 = header_string.split_ascii_whitespace();
    let x = l1.next().throw()?;
    let target = l1.next().throw()?;
    assert!(x == "PUT");
    let mut cl = 0;
    loop {
        let s = read_line(stream).await?;
        let s = str::from_utf8(&s)?;
        let xs = s.split_once(":");
        if let Some((start, remainder)) = xs {
            if start == "Content-Length" {
                cl = remainder.trim().parse::<usize>()?;
            }
        }
        if s.len() == 0 {
            break;
        }
    }
    let mut buf = Vec::with_capacity(cl);
    for _ in 0..cl {
        buf.push(0);
    }
    _ = stream.read_exact(&mut buf).await?;
    Ok(HTTPRequest::Put {
        target: target.into(),
        msg: buf.into(),
    })
}

pub async fn http_parse_connect(
    header_string: &str,
    stream: &mut TcpStream,
) -> Throws<HTTPRequest> {
    let mut l1 = header_string.split_ascii_whitespace();
    let x = l1.next().throw()?;
    let target = l1.next().throw()?;
    assert!(x == "CONNECT");
    let mut cl = 0;
    loop {
        let s = read_line(stream).await?;
        let s = str::from_utf8(&s)?;
        let xs = s.split_once(":");
        if let Some((start, remainder)) = xs {
            if start == "Content-Length" {
                cl = remainder.trim().parse::<usize>()?;
            }
        }
        if s.len() == 0 {
            break;
        }
    }
    let mut buf = Vec::with_capacity(cl);
    for _ in 0..cl {
        buf.push(0);
    }
    _ = stream.read_exact(&mut buf).await?;
    Ok(HTTPRequest::Connect {
        target: target.into(),
        msg: buf.into(),
    })
}

pub async fn http_parse_delete(header_string: &str, stream: &mut TcpStream) -> Throws<HTTPRequest> {
    let mut l1 = header_string.split_ascii_whitespace();
    let x = l1.next().throw()?;
    let target = l1.next().throw()?;
    assert!(x == "DELETE");
    let mut cl = 0;
    loop {
        let s = read_line(stream).await?;
        let s = str::from_utf8(&s)?;
        let xs = s.split_once(":");
        if let Some((start, remainder)) = xs {
            if start == "Content-Length" {
                cl = remainder.trim().parse::<usize>()?;
            }
        }
        if s.len() == 0 {
            break;
        }
    }
    let mut buf = Vec::with_capacity(cl);
    for _ in 0..cl {
        buf.push(0);
    }
    Ok(HTTPRequest::Delete {
        target: target.into(),
        msg: buf.into(),
    })
}


#[derive(Debug, Clone)]
pub struct HttpConfig{
    pub handle_gets_locally:bool,
    pub serve_dir:String,
}

pub struct HttpServer<T:ThreadSafeIsh>{
    events:EventSync<T>,
    listener:TcpListener,
    config:HttpConfig,
    _forwarder:BPipe<T>,
}

#[async_trait]
impl<T:ThreadSafeIsh+Clone> Daemon for HttpServer<T>{
    async fn run(&mut self) {
       loop{
            let con = tokio::time::timeout(Duration::from_millis(250), self.listener.accept()).await; 
            let Ok(con) = con else{
                self.update().await;
                continue;
            };
            match con{
                Ok((con, addr))=>{ 
                    let con_id = TcpConnectionId::alloc();
                    self.events.new_event_global(Event::NotifyNewTcpConnection { id: con_id }).unwrap();
                    let ev = self.events.clone();
                    let cfg = self.config.clone();
                    tokio::spawn(async move{
                        Self::run_tcp_connection(con, con_id,addr,ev, cfg).await
                    });
                }
                Err(e)=>{
                    println!("error:{}",e);
                    self.update().await; 
                    todo!()
                }
            }
        } 
    }
}


impl<T:ThreadSafeIsh+Clone> HttpServer<T>{
    pub async fn try_new(addr:&str, config:HttpConfig, events:EventSync<T>)->Throws<Self>{
        let forwarder = EventForwarder::new(|_|{
            true
        } ,events.clone()).await;
        let listener = TcpListener::bind(addr).await?;
        Ok(Self {events, listener ,config, _forwarder:forwarder})
    }
    
    pub async fn update(&mut self){
        
    }

    pub async fn run_tcp_connection(con:TcpStream, con_id:TcpConnectionId,addr:std::net::SocketAddr, sync:EventSync<T>, config:HttpConfig){ 
        _= addr;
        let events = EventForwarder::new_globals(|f|{
            let tmp = f.get_type();
            tmp == EventType::HttpResponse  || tmp ==EventType::NetOutput || tmp == EventType::NetInput || tmp == EventType::NotifyNewTcpConnection
        },sync.clone()).await;
        let mut handler = TcpHandler{events, stream:con, con_id, config, sync:sync.clone()};
        handler.run().await;
    }
}

struct TcpHandler<T:ThreadSafeIsh+Clone>{
    events:BPipe<Event<T>>, 
    stream:TcpStream,
    con_id:TcpConnectionId,
    config:HttpConfig,
    sync:EventSync<T>,
}
impl<T:ThreadSafeIsh+Clone> TcpHandler<T>{
    async fn poll_events(&mut self)->Throws<()>{
        loop{
            let Some(ev )= self.events.recieve().unwrap()else {
                break;
            };
          //  println!("event:{:#?}", ev.get_type());
            match ev{
                Event::HttpResponse { id, response }=>{
                   if id == self.con_id{
                        http_response_write(&mut self.stream, response).await?;
                   } 
                }
                Event::NetOutput { id, data }=>{
                    if id == self.con_id{
                        self.stream.write(&data).await?;
                    }
                }
                Event::NotifyNewTcpConnection { id }=>{
                        println!("cond_id:{},noted the creation of:{}", self.con_id.inner(),id.inner()); 
                }
                _=>{
                    println!("cond_id:{},unhandled event:{:#?}", self.con_id.inner(),ev.get_type());
                }
            } 
        }
        Ok(())
    }

    async fn run(&mut self){ 
        loop{
            let _e = self.poll_events().await;
            let err_req = http_get_request(&mut self.stream).await;
            match err_req{
                Ok(req) => {
                    self.handle_request(req).await.unwrap();
                }
                Err(x) => {
                    let line =x.line;
                    let f = x.file;
                    println!("threw exception:{} line:{} file:{}", x.get_error(),line, f);
                }
            }
        }
    }

    async fn handle_request(&mut self, req:HTTPRequest)->Throws<()>{
        match &req{
            HTTPRequest::Get { target, msg:_ } => {
                if self.config.handle_gets_locally{
                    let path =self.config.serve_dir.clone()+ if target.as_ref() != "/"{ target}else {"/index.html" };
                    let p = std::path::Path::new(&path);
                    let Ok(p2) = p.canonicalize()else{
                        self.stream.write(b"HTTP/1.1 404 Not Found\r\n").await?;
                        return Ok(());
                    };
                    let ancestors:HashSet<_> = p2.ancestors().collect();
                    let sd_path =  std::path::Path::new(&self.config.serve_dir).canonicalize()?;
                    if !ancestors.contains(sd_path.as_path()){
                        println!("forbidden:{:#?}", p2.to_str().unwrap());
                        self.stream.write(b"HTTP/1.1 403 Forbidden\r\n").await?;
                        return Ok(());
                    }
                    let ext = p.extension();
                     let s = std::fs::read(p2)?;
                    match ext{
                        Some(c)=>{
                            match c.to_str().throw()?{
                                "txt"=>{
                                    http_write_string_response(&mut self.stream, &s).await?;
                                }
                                ""=>{
                                    http_write_string_response(&mut self.stream, &s).await?;
                                }
                                "html"=>{
                                    http_write_html_response(&mut self.stream, &s).await?;
                                }
                                "png"=>{
                                    http_write_png_response(&mut self.stream, &s).await?;
                                }
                                "jpg"=>{
                                    http_write_jpg_response(&mut self.stream, &s).await?;
                                }
                                "jpeg"=>{
                                    http_write_jpg_response(&mut self.stream, &s).await?;
                                }
                                "js"=>{
                                    http_write_js_response(&mut self.stream, &s).await?;
                                }
                                "json"=>{
                                    http_write_json_response(&mut self.stream, &s).await?;
                                }
                                _=>{
                                    todo!()
                                }
                            }
                        }
                        None=>{
                            http_write_string_response(&mut self.stream, &s).await?;
                        }
                    } 
                }else{
                    let ev = Event::HttpRequest { id: self.con_id, request: req.clone() };
                    self.sync.new_event_global(ev)?; 
                }
            }
            HTTPRequest::Head { target, msg:_ } => {
                if self.config.handle_gets_locally{
                    let path =self.config.serve_dir.clone()+ if target.as_ref() != "/"{ target}else {"/index.html" };
                    let p = std::path::Path::new(&path);
                    let Ok(p2) = p.canonicalize()else{
                        self.stream.write(b"HTTP/1.1 404 Not Found\r\n").await?;
                        return Ok(());
                    };
                    let ancestors:HashSet<_> = p2.ancestors().collect();
                    if !ancestors.contains( std::path::Path::new(&self.config.serve_dir)){

                        self.stream.write(b"HTTP/1.1 403 Forbidden\r\n").await?;
                        return Ok(());
                    }
                    let ext = p.extension();
                    let s = std::fs::read(p2)?;
                    let content_type=  match ext{
                        Some(c)=>{
                            match c.to_str().throw()?{
                                "txt"=>{
                                    "text/plaintext"
                                }
                                ""=>{
                                    "text/plaintext"
                                }
                                "html"=>{
                                    "text/html"
                                }
                                "png"=>{
                                    "image/png"
                                }
                                "jpg"=>{
                                    "image/jpeg"
                                }
                                "jpeg"=>{
                                    "image/jpeg"
                                }
                                "js"=>{
                                    "text/javascript"
                                }
                                "json"=>{
                                    "text/json"
                                }
                                _=>{
                                    todo!()
                                }
                            }
                        }
                        None=>{
                            "text/plaintext"
                        }
                    } ;
                    let f = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type:{}\r\ncharset=UTF-8\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\n",content_type,
                    s.len());
                    self.stream.write(f.as_bytes()).await?;
                }else{
                    let ev = Event::HttpRequest { id: self.con_id, request: req.clone() };
                    self.sync.new_event_global(ev)?; 
                }
            }
            _=>{
                let ev = Event::HttpRequest { id: self.con_id, request: req.clone() };
                self.sync.new_event_global(ev)?;
            }
        }
        Ok(())
    }
}

pub async fn http_response_write(stream: &mut TcpStream, response:HTTPResponse)->Throws<()>{
    match response.response_type{
        HttpResponseType::Text => {http_write_string_response(stream, &response.data).await?;}
        HttpResponseType::Html => {http_write_html_response(stream, &response.data).await?;},
        HttpResponseType::Png => {http_write_png_response(stream, &response.data).await?;}
        HttpResponseType::Jpeg => {http_write_jpg_response(stream, &response.data).await?;}
        HttpResponseType::Json => {http_write_json_response(stream, &response.data).await?;}
        HttpResponseType::Js => {http_write_js_response(stream, &response.data).await?;}
    }
    Ok(())
}

pub struct HttpConfigBuilder{
    handle_get_requests_locally:bool,
    serve_dir:String,
}

impl HttpConfig{
    pub fn new()->HttpConfigBuilder{
        HttpConfigBuilder {  handle_get_requests_locally:true, serve_dir:String::from(".")}
    }
}

impl HttpConfigBuilder{  
    pub fn build(self)->HttpConfig{
       HttpConfig { serve_dir:self.serve_dir, handle_gets_locally:self.handle_get_requests_locally } 
    }
    pub fn forward_gets(mut self)->Self{
        self.handle_get_requests_locally = true;
        self
    }
    pub fn serve_dir(mut self, dir:impl Into<String>)->Self{
        self.serve_dir = dir.into();
        self
    }
}

#[derive(Debug, Clone)]
pub struct ServerEvent{

}

pub async fn file_server(serve_dir:&str, addr:String){
    let (_sender, mut handler) = EventHandler::<ServerEvent>::new();
    let dir = serve_dir.to_string();
    handler.run(async move |sync|{file_server_setup(sync, dir.clone(), addr.clone()).await}).await;
}

pub async fn file_server_setup(sync:EventSync<ServerEvent>, serve_dir:String, addr:String){
    let config = HttpConfig::new().serve_dir(serve_dir).build();
    let server = HttpServer::try_new(&addr, config,sync.clone()).await.unwrap();
    sync.new_event_global(Event::CreateDaemon { daemon: Box::new(server), id: DaemonId::alloc() }).unwrap();
}