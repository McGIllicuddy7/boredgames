pub mod rtils_useful {
    #[allow(unused)]
    use std::backtrace;
    use std::collections::VecDeque;
    use std::error::Error;
    use std::fmt::Display;

    use std::io::{Read, Write};
    use std::str::FromStr;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex, RwLock};
    use std::task::Poll;
    use std::thread::yield_now;

    use serde::{Deserialize, Serialize};
    pub trait CopyFromStr {
        fn copy_from_str(&mut self, string: &str) -> bool;
    }

    impl<T: FromStr> CopyFromStr for T
    where
        <T as FromStr>::Err: std::fmt::Debug,
    {
        fn copy_from_str(&mut self, string: &str) -> bool {
            let s = Self::from_str(string);
            match s {
                Ok(s) => {
                    *self = s;
                    true
                }
                Err(e) => {
                    println!("{:#?} in <{:#?}>", e, string);
                    false
                }
            }
        }
    }
    pub fn dyn_scanf(input: &str, format: &str, args: &mut [&mut dyn CopyFromStr]) -> bool {
        let mut args_index = 0;
        let mut fmt = format.chars();
        let mut inp = input.chars();
        let mut done = false;
        loop {
            let Some(i) = fmt.next() else {
                break;
            };
            let Some(j) = inp.next() else {
                return false;
            };
            if i == '{' {
                let Some(n) = fmt.next() else {
                    return false;
                };
                if n == '}' {
                    let end_delim = fmt.next();
                    let mut argument = String::new();
                    argument.push(j);
                    'inner: loop {
                        let j = inp.next();
                        if j == end_delim {
                            if j.is_none() {
                                done = true;
                            }
                            break 'inner;
                        } else {
                            if j.is_none() {
                                return false;
                            }
                            argument.push(j.unwrap());
                        }
                    }
                    if let Some(e) = end_delim
                        && (e == '{' || e == '}')
                    {
                        let Some(n) = fmt.next() else {
                            return false;
                        };
                        if n != e {
                            return false;
                        }
                    }
                    if args_index >= args.len() {
                        return false;
                    }
                    let rs = args[args_index].copy_from_str(&argument);
                    if !rs {
                        return false;
                    }
                    args_index += 1;
                } else if n == '{' {
                    if j != '{' {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if i == '}' {
                let Some(n) = fmt.next() else {
                    return false;
                };
                if n != '}' {
                    println!("{:#?}", n);
                    return false;
                }
                if j != '}' {
                    return false;
                }
            } else if i != j {
                return false;
            }
            if done {
                break;
            }
        }
        if inp.next().is_some() {
            return false;
        }
        if args_index != args.len() {
            return false;
        }
        true
    }
    mod rtils {
        #[allow(unused)]
        pub use crate::*;
    }
    #[macro_export]
    macro_rules! sscanf {
    ($input:expr, $fmt:literal) => {
        dyn_scanf(($input).as_str(), $fmt, &mut [])
    };
    ($input:expr, $fmt:literal, $($args:expr),+) => {
        dyn_scanf(($input).as_str(), $fmt, &mut [$(&mut $args), +])
    };
}

    #[macro_export]
    macro_rules! MAKE_INTO_ERROR {
        ($t:ty) => {
            impl ::std::fmt::Display for $t {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    write!(f, "{:#?}", self)
                }
            }
            impl ::std::error::Error for $t {}
        };
    }

    #[derive(Debug)]
    pub struct Exception {
        pub trace: std::backtrace::Backtrace,
        pub error: Box<dyn std::error::Error + Send + Sync + 'static>,
        pub file: &'static str,
        pub line: u32,
    }
    impl Display for Exception {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "Exception:{}
            thrown:
                file:{}
                line:{}
            in:
                {}",
                self.error, self.file, self.line, self.trace
            )
        }
    }

    impl Exception {
        pub fn get_error(self) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self.error
        }
        pub fn error_as<T: std::error::Error + Send + Sync + 'static>(
            self,
        ) -> Result<Box<T>, Box<dyn std::error::Error + Send + Sync + 'static>> {
            self.error.downcast()
        }
    }

    #[derive(Debug)]
    pub struct BadOption {}
    MAKE_INTO_ERROR!(BadOption);

    pub trait Throw<T> {
        fn throw(self) -> Result<T, Exception>;
    }

    impl<U, T: std::error::Error + Send + Sync + 'static> Throw<U> for Result<U, T> {
        fn throw(self) -> Result<U, Exception> {
            match self {
                Ok(x) => Ok(x),
                Err(e) => Err(Exception {
                    trace: backtrace::Backtrace::capture(),
                    error: Box::new(e),
                    file: file!(),
                    line: line!(),
                }),
            }
        }
    }

    impl<U> Throw<U> for Option<U> {
        fn throw(self) -> Result<U, Exception> {
            match self {
                Some(x) => Ok(x),
                None => Err(Exception {
                    trace: backtrace::Backtrace::capture(),
                    error: Box::new(BadOption {}),
                    file: file!(),
                    line: line!(),
                }),
            }
        }
    }

    #[cfg(debug_assertions)]
    #[macro_export]
    macro_rules! throw {
        ($exp:expr) => {
            return Err(Exception {
                trace: ::std::backtrace::Backtrace::force_capture(),
                error: Box::from($exp),
                file: file!(),
                line: line!(),
            })
        };
    }
    #[cfg(not(debug_assertions))]
    #[macro_export]
    macro_rules! throw {
        ($exp:expr) => {
            return Err(Exception {
                trace: ::std::backtrace::Backtrace::capture(),
                error: Box::from($exp),
                file: file!(),
                line: line!(),
            })
        };
    }

    #[cfg(debug_assertions)]
    #[macro_export]
    macro_rules! new_exception {
        ($exp:expr) => {
            Exception {
                trace: ::std::backtrace::Backtrace::force_capture(),
                error: Box::from($exp),
                file: file!(),
                line: line!(),
            }
        };
    }
    #[cfg(not(debug_assertions))]
    #[macro_export]
    macro_rules! new_exception {
        ($exp:expr) => {
            Exception {
                trace: ::std::backtrace::Backtrace::capture(),
                error: Box::from($exp),
                file: file!(),
                line: line!(),
            }
        };
    }

    #[macro_export]
    macro_rules! try_catch {
    (try $block:block catch ($err:ident) $if_err:block) => {
        {

        let mut f =|| {
            $block
            #[allow(unused)]
            Ok::<(), Exception>(())
        };
        if let Err($err) = (f)() $if_err
        }
    };
}

    pub type Throws<T> = Result<T, Exception>;
    impl<T: Into<Box<dyn std::error::Error + Send + Sync + 'static>>> From<T> for Exception {
        fn from(value: T) -> Self {
            new_exception!(value.into())
        }
    }
    #[cfg(feature = "net")]
    pub mod net {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::{TcpListener, TcpStream},
        };
        /* HTTP request enum, we will accept a body in any request for compatability with dumbshit because I
         * am a good girl
         * */
        use super::*;
        use tokio::net::TcpStream;

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
            loop {
                let header = read_line(stream).await?;
                let header_string = std::str::from_utf8(&header)?;
                let mut args = header_string.split_ascii_whitespace();
                if args.clone().count() == 0 {
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

        pub async fn http_parse_get(
            header_string: &str,
            stream: &mut TcpStream,
        ) -> Throws<HTTPRequest> {
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

        pub async fn http_parse_head(
            header_string: &str,
            stream: &mut TcpStream,
        ) -> Throws<HTTPRequest> {
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

        pub async fn http_parse_post(
            header_string: &str,
            stream: &mut TcpStream,
        ) -> Throws<HTTPRequest> {
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

        pub async fn http_parse_put(
            header_string: &str,
            stream: &mut TcpStream,
        ) -> Throws<HTTPRequest> {
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

        pub async fn http_parse_delete(
            header_string: &str,
            stream: &mut TcpStream,
        ) -> Throws<HTTPRequest> {
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

            Out { v: self.v.clone() }
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
            if self.done.load(std::sync::atomic::Ordering::Acquire) {
                return Err("done".into());
            }
            let mut sending = self.sending.lock().unwrap();
            sending.push_back(v);
            Ok(())
        }

        pub fn recieve(&self) -> Throws<Option<T>> {
            if self.done.load(std::sync::atomic::Ordering::Acquire) {
                todo!()
            }
            let mut recieving = self.recieving.lock().unwrap();
            Ok(recieving.pop_front())
        }

        pub fn receive_buffer(&self) -> Throws<Vec<T>> {
            let mut out = Vec::new();
            while let Some(x) = self.recieve()? {
                out.push(x);
            }
            Ok(out)
        }

        pub fn recieve_wait(&self) -> Throws<T> {
            loop {
                if self.done.load(std::sync::atomic::Ordering::Acquire) {
                    return Err("done".into());
                }
                let mut recieving = self.recieving.lock().unwrap();
                if let Some(x) = recieving.pop_front() {
                    return Ok(x);
                }
                yield_now();
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
                    if self.done.load(std::sync::atomic::Ordering::Acquire) {
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
    impl<T> Iterator for BPipe<T> {
        type Item = Throws<T>;

        fn next(&mut self) -> Option<Self::Item> {
            let tmp = self.recieve();
            match tmp {
                Err(e) => Some(Err(e)),
                Ok(x) => x.map(Ok),
            }
        }
    }
    impl<T> Drop for BPipe<T> {
        fn drop(&mut self) {
            self.done.store(true, std::sync::atomic::Ordering::Release);
        }
    }

    pub fn stream_write_bytes(stream: &mut std::net::TcpStream, bytes: &[u8]) -> Throws<()> {
        let blen = (bytes.len() as u64).to_le_bytes();
        let _ = stream.write(&blen)?;
        let _ = stream.write(bytes)?;
        Ok(())
    }

    pub fn stream_try_read_bytes(stream: &mut std::net::TcpStream) -> Throws<Option<Vec<u8>>> {
        stream.set_nonblocking(true)?;
        let mut bytes = [0; 8];
        let e = stream.read_exact(&mut bytes);
        if let Err(e) = e {
            match e.kind() {
                std::io::ErrorKind::WouldBlock => {
                    return Ok(None);
                }
                _ => {
                    throw!(e);
                }
            }
        }
        let len = u64::from_le_bytes(bytes);
        let mut buf = Vec::new();
        buf.reserve_exact(len as usize);
        buf.extend(std::iter::repeat_n(0, len as usize));
        stream.set_nonblocking(false)?;
        let e = stream.read_exact(&mut buf);
        stream.set_nonblocking(true)?;
        if let Err(e) = e {
            throw!(e);
        }
        Ok(Some(buf))
    }

    pub fn stream_read_bytes_blocking(stream: &mut std::net::TcpStream) -> Throws<Vec<u8>> {
        stream.set_nonblocking(false)?;
        let mut bytes = [0; 8];
        let e = stream.read_exact(&mut bytes);
        if let Err(e) = e {
            stream.set_nonblocking(true)?;
            throw!(e);
        }
        let len = u64::from_le_bytes(bytes);
        let mut buf = Vec::new();
        buf.reserve_exact(len as usize);
        buf.extend(std::iter::repeat_n(0, len as usize));
        let e = stream.read_exact(&mut buf);
        stream.set_nonblocking(true)?;
        if let Err(e) = e {
            throw!(e);
        }
        Ok(buf)
    }

    pub fn stream_read_bytes_async(
        stream: &mut std::net::TcpStream,
    ) -> impl Future<Output = Throws<Vec<u8>>> {
        struct Waiting<'a> {
            stream: &'a mut std::net::TcpStream,
        }
        impl<'a> Future for Waiting<'a> {
            type Output = Throws<Vec<u8>>;
            fn poll(
                mut self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
            ) -> Poll<Self::Output> {
                self.stream.set_nonblocking(true)?;
                let mut bytes = [0; 8];
                let e = self.stream.read_exact(&mut bytes);
                if let Err(e) = e {
                    match e.kind() {
                        std::io::ErrorKind::WouldBlock => Poll::Pending,
                        _ => Poll::Ready(Err(new_exception!(e))),
                    }
                } else {
                    self.stream.set_nonblocking(false).unwrap();
                    let len = u64::from_le_bytes(bytes);
                    let mut buf = Vec::new();
                    buf.reserve_exact(len as usize);
                    buf.extend(std::iter::repeat_n(0, len as usize));
                    let _ = self.stream.read_exact(&mut buf);
                    self.stream.set_nonblocking(true)?;
                    Poll::Ready(Ok(buf))
                }
            }
        }
        Waiting { stream }
    }

    #[repr(transparent)]
    #[derive(Clone, Debug, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    pub struct Immutable<T> {
        x: T,
    }
    impl<T> Immutable<T> {
        pub fn new(x: T) -> Self {
            Self { x }
        }
        pub fn get(&self) -> &T {
            &self.x
        }

        /// # Safety
        ///
        /// Must ensure that whatever invariants of the type holding this and the world at large are upheld, this is a public function but it should only be used in private functions of other types.
        ///
        pub unsafe fn get_mut(&mut self) -> &mut T {
            &mut self.x
        }
        pub fn take(self) -> T {
            self.x
        }
    }

    #[derive(Clone, Debug)]
    pub struct Token {
        pub text: String,
        pub file: String,
        pub line: usize,
    }

    #[derive(Clone, Debug)]
    pub struct TokenStream {
        pub tokens: Vec<Token>,
        pub index: usize,
    }

    impl AsRef<str> for Token {
        fn as_ref(&self) -> &str {
            &self.text
        }
    }

    impl TokenStream {
        pub fn from_string(s: String, file: String) -> Self {
            let tokens = tokenize(s, file);
            Self { tokens, index: 0 }
        }
        pub fn peek(&self) -> Option<Token> {
            let mut t = self.clone();
            t.next()
        }
        pub fn insert_next(&mut self, t: Token) {
            self.tokens.insert(self.index, t);
        }
    }

    impl Iterator for TokenStream {
        type Item = Token;
        fn next(&mut self) -> Option<Self::Item> {
            if self.index < self.tokens.len() {
                let out = self.tokens[self.index].clone();
                self.index += 1;
                Some(out)
            } else {
                None
            }
        }
    }

    pub fn tokenize(s: String, file: String) -> Vec<Token> {
        enum State {
            Whitespace,
            Ident,
            String,
            Comment,
            StringEscaped,
        }
        let mut out = Vec::new();
        let mut buf = String::new();
        let mut line = 1;
        let mut state = State::Whitespace;
        for c in s.chars() {
            match state {
                State::Whitespace => {
                    if c == ' ' || c == '\t' {
                    } else if c == '\n' {
                        line += 1;
                    } else if c == ':'
                        || c == '+'
                        || c == '-'
                        || c == '*'
                        || c == '/'
                        || c == '('
                        || c == ')'
                        || c == '<'
                        || c == '>'
                    {
                        out.push(Token {
                            text: c.to_string(),
                            file: file.clone(),
                            line,
                        });
                    } else if c == '"' {
                        buf = String::new();
                        state = State::String;
                    } else if c == ';' {
                        buf = String::new();
                        state = State::Comment;
                    } else {
                        buf = String::new();
                        buf.push(c);
                        state = State::Ident;
                    }
                }
                State::Ident => {
                    if !c.is_whitespace()
                        && !(c == ':'
                            || c == '+'
                            || c == '-'
                            || c == '*'
                            || c == '/'
                            || c == ';'
                            || c == '('
                            || c == ')'
                            || c == '>'
                            || c == '<')
                    {
                        buf.push(c);
                    } else {
                        out.push(Token {
                            text: buf,
                            file: file.clone(),
                            line,
                        });
                        buf = String::new();
                        if c == '\n' {
                            line += 1;
                        } else if c == ':'
                            || c == '+'
                            || c == '-'
                            || c == '*'
                            || c == '/'
                            || c == '('
                            || c == ')'
                            || c == '>'
                            || c == '<'
                        {
                            out.push(Token {
                                text: c.to_string(),
                                file: file.clone(),
                                line,
                            });
                        }
                        state = if c == ';' {
                            State::Comment
                        } else {
                            State::Whitespace
                        };
                    }
                }
                State::String => {
                    if c == '"' {
                        buf = "\"".to_string() + &buf + "\"";
                        out.push(Token {
                            text: buf,
                            file: file.clone(),
                            line,
                        });
                        buf = String::new();
                        state = State::Whitespace;
                    } else if c == '\\' {
                        state = State::StringEscaped;
                    } else if c == '\n' {
                        line += 1;
                    } else {
                        buf.push(c);
                    }
                }
                State::StringEscaped => {
                    buf.push(c);
                    state = State::String;
                }
                State::Comment => {
                    if c == '\n' {
                        line += 1;
                        state = State::Whitespace
                    }
                }
            }
        }
        if !buf.is_empty() {
            out.push(Token {
                text: buf,
                file,
                line,
            });
        }
        //println!("{:#?}", out);
        out
    }

    pub struct Shared<T> {
        inner: Arc<RwLock<T>>,
    }
    impl<T> Shared<T> {
        pub fn new(value: T) -> Self {
            Self {
                inner: Arc::new(RwLock::new(value)),
            }
        }
        pub fn shared_store_copy(&self, v: T) {
            *self.inner.write().unwrap() = v;
        }
    }
    impl<T: Clone> Shared<T> {
        pub fn shared_get_copy(&self) -> T {
            self.inner.read().unwrap().clone()
        }
    }

    #[macro_export]
    macro_rules! make_shared_type {
    ($T:ty, $((fn $name:ident(&mut self$(,)? $($arg:ident:$ty:ty$(,)?)*)->$returns:ty))* , $((fn $const_name:ident(& self$(,)? $($const_arg:ident:$const_ty:ty$(,)?)*)->$const_returns:ty))*)=> {
        impl Shared<$T>{
            $(
                pub fn $name(&self, $($arg:$ty)*)->$returns{
                    let mut lock = self.inner.write().unwrap();
                    lock.$name($($arg,)*)
                }
            )*
            $(
                pub fn $const_name(&self, $($const_arg:$const_ty)*)->$const_returns{
                    let lock = self.inner.read().unwrap();
                    lock.$const_name($($const_arg,)*)
                }
            )*
        }
    };
}

    use std::{
        borrow::Borrow,
        cell::{Cell, UnsafeCell},
        fmt::{Debug, Formatter},
        hash::{DefaultHasher, Hash, Hasher},
        ops::{Deref, DerefMut, Index, IndexMut},
        sync::atomic::{
            AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize, AtomicPtr, AtomicU8, AtomicU16,
            AtomicU32, AtomicU64, AtomicUsize,
        },
    };

    pub trait Trivial {
        const IS_TRIVIAL: bool = const {
            if std::mem::needs_drop::<Self>() {
                panic!("type is drop");
            } else {
                true
            }
        };
        ///
        /// # Safety
        ///
        /// DO NOT MANUALLY IMPLEMENT THIS FUNCTION PLEASE
        unsafe fn no_drop_impl(&self) {
            println!("{}", Self::IS_TRIVIAL);
        }
    }
    pub trait TrivialClone: Clone + Trivial {}
    impl<T: Trivial + Clone> TrivialClone for T {}

    impl<T: Trivial, U: Trivial> Trivial for (T, U) {}
    impl<T: Trivial, U: Trivial, V: Trivial> Trivial for (T, U, V) {}
    impl<T: Trivial, U: Trivial, V: Trivial, W: Trivial> Trivial for (T, U, V, W) {}
    impl<T: Trivial, U: Trivial, V: Trivial, W: Trivial, X: Trivial> Trivial for (T, U, V, W, X) {}
    impl<T: Trivial, U: Trivial, V: Trivial, W: Trivial, X: Trivial, Y: Trivial> Trivial
        for (T, U, V, W, X, Y)
    {
    }
    impl<T: Trivial, U: Trivial, V: Trivial, W: Trivial, X: Trivial, Y: Trivial, Z: Trivial> Trivial
        for (T, U, V, W, X, Y, Z)
    {
    }
    impl Trivial for usize {}
    impl Trivial for u8 {}
    impl Trivial for u16 {}
    impl Trivial for u32 {}
    impl Trivial for u64 {}
    impl Trivial for u128 {}
    impl Trivial for isize {}
    impl Trivial for i8 {}
    impl Trivial for i16 {}
    impl Trivial for i32 {}
    impl Trivial for i64 {}
    impl Trivial for i128 {}
    impl Trivial for f64 {}
    impl Trivial for f32 {}
    impl Trivial for bool {}

    impl Trivial for AtomicBool {}
    impl Trivial for AtomicUsize {}
    impl Trivial for AtomicU8 {}
    impl Trivial for AtomicU16 {}
    impl Trivial for AtomicU32 {}
    impl Trivial for AtomicU64 {}
    impl Trivial for AtomicIsize {}
    impl Trivial for AtomicI8 {}
    impl Trivial for AtomicI16 {}
    impl Trivial for AtomicI32 {}
    impl Trivial for AtomicI64 {}

    impl<T> Trivial for *const T {}
    impl<T> Trivial for *mut T {}
    impl<T> Trivial for AtomicPtr<T> {}
    impl<T> Trivial for UnsafeCell<T> {}
    impl<T> Trivial for Cell<T> {}
    impl<const COUNT: usize, T: Trivial> Trivial for [T; COUNT] {}
    impl<T: ?Sized> Trivial for &T {}
    impl<T> Trivial for &mut T {}
    impl Trivial for () {}
    pub struct SpinLock<T> {
        cell: UnsafeCell<T>,
        lock: AtomicBool,
    }
    impl<T: Trivial> Trivial for SpinLock<T> {}

    impl<T> SpinLock<T> {
        pub fn new(value: T) -> Self {
            Self {
                cell: UnsafeCell::new(value),
                lock: AtomicBool::new(false),
            }
        }

        unsafe fn mark_locked(&self) {
            while self
                .lock
                .compare_exchange_weak(
                    false,
                    true,
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                )
                .is_err()
            {
                std::hint::spin_loop();
                std::thread::yield_now();
            }
        }

        unsafe fn mark_unlocked(&self) {
            self.lock.store(false, std::sync::atomic::Ordering::SeqCst);
        }

        unsafe fn try_mark_locked(&self) -> bool {
            self.lock
                .compare_exchange(
                    false,
                    true,
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                )
                .is_ok()
        }

        pub fn lock<'a>(&'a self) -> Lock<'a, T> {
            unsafe {
                self.mark_locked();
                Lock { inner: self }
            }
        }

        pub fn try_lock<'a>(&'a self) -> Option<Lock<'a, T>> {
            unsafe {
                if self.try_mark_locked() {
                    Some(Lock { inner: self })
                } else {
                    None
                }
            }
        }

        pub fn store(&self, value: T) {
            let mut lock = self.lock();
            *lock = value;
        }

        pub fn try_store(&self, value: T) -> Option<T> {
            if let Some(mut lock) = self.try_lock() {
                *lock = value;
                None
            } else {
                Some(value)
            }
        }
    }
    impl<T: Default> SpinLock<T> {
        pub fn take(&self) -> T {
            let mut lock = self.lock();
            let mut def = Default::default();
            std::mem::swap(&mut def, &mut *lock);
            def
        }

        pub fn try_take(&self) -> Option<T> {
            let mut lock = self.try_lock()?;
            let mut def = Default::default();
            std::mem::swap(&mut def, &mut *lock);
            Some(def)
        }
    }

    impl<T: Clone> SpinLock<T> {
        pub fn get(&self) -> T {
            let lock = self.lock();
            lock.clone()
        }

        pub fn try_get(&self) -> Option<T> {
            let lock = self.try_lock()?;
            Some(lock.clone())
        }
    }

    impl<T: Clone> Clone for SpinLock<T> {
        fn clone(&self) -> Self {
            let lock = self.lock();
            Self {
                cell: UnsafeCell::new(lock.clone()),
                lock: AtomicBool::new(false),
            }
        }
    }

    unsafe impl<T: Send> Send for SpinLock<T> {}
    unsafe impl<T: Sync> Sync for SpinLock<T> {}
    //impl<T: Trivial> Trivial for SpinLock<T> {}
    pub struct Lock<'a, T> {
        inner: &'a SpinLock<T>,
    }
    impl<'a, T> Drop for Lock<'a, T> {
        fn drop(&mut self) {
            unsafe {
                self.inner.mark_unlocked();
            }
        }
    }
    impl<'a, T> Deref for Lock<'a, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            unsafe { self.inner.cell.get().as_ref().unwrap() }
        }
    }

    impl<'a, T> DerefMut for Lock<'a, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            unsafe { self.inner.cell.get().as_mut().unwrap() }
        }
    }

    pub struct Arena {
        buffer: Arc<UnsafeCell<[u8]>>,
        next_ptr: Cell<usize>,
        next: UnsafeCell<Option<Arc<Arena>>>,
        lock: Mutex<()>,
    }
    impl Default for Arena {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Arena {
        pub fn new() -> Self {
            let count = 4096 * 4096;
            let mut v = Vec::new();
            v.reserve_exact(count);
            v.extend(std::iter::repeat_n(0, count));
            let buf1: Arc<[u8]> = v.into();
            let buf2 = unsafe { Arc::from_raw(Arc::into_raw(buf1) as *const UnsafeCell<[u8]>) };
            Self {
                buffer: buf2,
                next_ptr: Cell::new(0),
                lock: Mutex::new(()),
                next: UnsafeCell::new(None),
            }
        }
        pub fn new_sized(size: usize) -> Self {
            let count = if 4096 * 4096 > size {
                4096 * 4096
            } else {
                let mut tmp = 4096 * 4096;
                while tmp < size {
                    tmp += 4096 * 4096;
                }
                tmp
            };
            let mut v = Vec::new();
            v.reserve_exact(count);
            v.extend(std::iter::repeat_n(0, count));
            let buf1: Arc<[u8]> = v.into();
            let buf2 = unsafe { Arc::from_raw(Arc::into_raw(buf1) as *const UnsafeCell<[u8]>) };
            Self {
                buffer: buf2,
                next_ptr: Cell::new(0),
                lock: Mutex::new(()),
                next: UnsafeCell::new(None),
            }
        }

        #[allow(clippy::mut_from_ref)]
        pub fn alloc_bytes(&self, count: usize, align: usize) -> &mut [u8] {
            let _lock = self.lock.lock().unwrap();
            let len = count;
            let mut nxt = self.next_ptr.get();
            if !nxt.is_multiple_of(align) {
                nxt = nxt + align - nxt % align;
            }
            //safety, aligned pointer, guarrantees unique access to a location.
            unsafe {
                if nxt + len >= self.buffer.get().as_ref().unwrap().len() {
                    if let Some(next) = self.next.get().as_ref().unwrap() {
                        next.alloc_bytes(len, align)
                    } else {
                        let out = Arena::new_sized(count);
                        *self.next.get().as_mut().unwrap() = Some(Arc::new(out));
                        self.next
                            .get()
                            .as_ref()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .alloc_bytes(len, align)
                    }
                } else {
                    let out = &mut self.buffer.get().as_mut().unwrap()[nxt..nxt + len];
                    self.next_ptr.set(nxt + len);
                    out
                }
            }
        }

        #[allow(clippy::mut_from_ref)]
        pub fn alloc<T: Trivial>(&self, value: T) -> &mut T {
            assert!(T::IS_TRIVIAL);
            assert!(!std::mem::needs_drop::<T>());
            unsafe {
                let bytes = self.alloc_bytes(size_of_val(&value), align_of_val(&value));
                let obj = bytes.as_mut_ptr() as *mut T;
                obj.write(value);
                obj.as_mut().unwrap()
            }
        }

        pub fn debug_mem_usage(&self) -> usize {
            let _lock = self.lock.lock().unwrap();
            let base_count = self.next_ptr.get();
            unsafe {
                if let Some(nxt) = self.next.get().as_ref().unwrap() {
                    base_count + nxt.debug_mem_usage()
                } else {
                    base_count
                }
            }
        }
    }

    unsafe impl Send for Arena {}
    unsafe impl Sync for Arena {}
    #[derive(Clone)]
    pub enum List<'a, T: TrivialClone> {
        Empty(&'a Arena),
        Node(&'a ListNode<'a, T>),
    }
    impl<'a, T: TrivialClone> Trivial for List<'a, T> {}

    #[derive(Clone)]
    pub struct ListNode<'a, T: TrivialClone> {
        value: &'a T,
        next: List<'a, T>,
        arena: &'a Arena,
    }

    impl<'a, T: TrivialClone> Trivial for ListNode<'a, T> {}
    impl<'a, T: TrivialClone> List<'a, T> {
        pub fn new(arena: &'a Arena, value: T) -> &'a Self {
            let tmp = arena.alloc(ListNode {
                value: arena.alloc(value),
                next: List::Empty(arena),
                arena,
            });
            arena.alloc(Self::Node(tmp))
        }

        pub fn get_arena(&self) -> &'a Arena {
            match self {
                List::Empty(arena) => arena,
                List::Node(list_node) => list_node.arena,
            }
        }

        pub fn cons(&self, value: T) -> &'a Self {
            let ar = self.get_arena();
            let node = ar.alloc(ListNode {
                value: ar.alloc(value),
                next: self.clone(),
                arena: ar,
            });
            ar.alloc(List::Node(node))
        }

        pub fn car(&self) -> &'a T {
            match self {
                List::Empty(_) => todo!(),
                List::Node(list_node) => list_node.value,
            }
        }

        pub fn cdr(&self) -> Self {
            match self {
                List::Empty(ar) => List::Empty(ar),
                List::Node(list_node) => list_node.next.clone(),
            }
        }

        pub fn get(&self, index: usize) -> Option<&'a T> {
            let mut i = 0;
            let mut current = self.clone();
            while let Self::Node(n) = current {
                if i == index {
                    return Some(n.value);
                }
                i += 1;
                current = n.next.clone()
            }
            None
        }

        pub fn reverse(&self) -> &'a Self {
            let mut base: &'a List<'_, _> = self.get_arena().alloc(List::Empty(self.get_arena()));
            for i in self.clone() {
                base = base.cons(i);
            }
            base
        }

        pub const fn len(&self) -> usize {
            let mut out = 0;
            let mut next = self;
            while let Self::Node(n) = next {
                out += 1;
                next = &n.next;
            }
            out
        }

        pub const fn is_empty(&self) -> bool {
            self.len() == 0
        }
    }

    impl<'a, T: Debug + TrivialClone> Debug for List<'a, T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut dbg = f.debug_list();
            let slf = self.clone();
            for i in slf {
                dbg.entry(&i);
            }
            dbg.finish()
        }
    }

    impl<'a, T: TrivialClone> Iterator for List<'a, T> {
        type Item = T;

        fn next(&mut self) -> Option<Self::Item> {
            match self {
                List::Empty(_) => None,
                List::Node(list_node) => {
                    let out = Some(list_node.value.clone());
                    *self = list_node.next.clone();
                    out
                }
            }
        }
    }

    impl<'a, T: TrivialClone> Index<usize> for List<'a, T> {
        type Output = T;
        fn index(&self, index: usize) -> &Self::Output {
            self.get(index).unwrap()
        }
    }

    pub enum ListMut<'a, T: TrivialClone> {
        Empty(&'a Arena),
        Node(&'a mut ListNodeMut<'a, T>),
    }

    impl<'a, T: TrivialClone> Trivial for ListMut<'a, T> {}
    impl<'a, T: TrivialClone> Clone for ListMut<'a, T> {
        fn clone(&self) -> Self {
            match self {
                Self::Empty(ar) => Self::Empty(ar),
                Self::Node(x) => {
                    let tmp = &**x;
                    let tmp = tmp.arena.alloc(tmp.clone());
                    Self::Node(tmp)
                }
            }
        }
    }

    pub struct ListNodeMut<'a, T: TrivialClone> {
        value: &'a mut T,
        next: ListMut<'a, T>,
        arena: &'a Arena,
    }
    impl<'a, T: TrivialClone> Clone for ListNodeMut<'a, T> {
        fn clone(&self) -> Self {
            Self {
                value: self.arena.alloc(self.value.clone()),
                next: self.next.clone(),
                arena: self.arena,
            }
        }
    }
    impl<'a, T: TrivialClone> Trivial for ListNodeMut<'a, T> {}

    impl<'a, T: TrivialClone> ListMut<'a, T> {
        pub fn new(arena: &'a Arena, value: T) -> &'a Self {
            let tmp = arena.alloc(ListNodeMut {
                value: arena.alloc(value),
                next: ListMut::Empty(arena),
                arena,
            });
            arena.alloc(Self::Node(tmp))
        }

        pub fn get_arena(&self) -> &'a Arena {
            match self {
                ListMut::Empty(arena) => arena,
                ListMut::Node(list_node) => list_node.arena,
            }
        }

        pub fn cons(&self, value: T) -> &'a mut Self {
            let ar = self.get_arena();
            let node = ar.alloc(ListNodeMut {
                value: ar.alloc(value),
                next: self.clone(),
                arena: ar,
            });
            ar.alloc(ListMut::Node(node))
        }

        pub fn car(&'a mut self) -> &'a mut T {
            match self {
                ListMut::Empty(_) => todo!(),
                ListMut::Node(list_node) => list_node.value,
            }
        }

        pub fn cdr(self) -> &'a mut Self {
            match self {
                ListMut::Empty(ar) => ar.alloc(ListMut::Empty(ar)),
                ListMut::Node(list_node) => &mut list_node.next,
            }
        }

        pub fn get(&self, index: usize) -> Option<&T>
where {
            let mut i = 0;
            let mut current = self;
            while let ListMut::Node(c) = current {
                if i == index {
                    return Some(&*c.value);
                }
                i += 1;
                current = &c.next;
            }
            None
        }
        pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
            let mut i = 0;
            let mut current = self;
            while let ListMut::Node(c) = current {
                if i == index {
                    return Some(&mut *c.value);
                }
                i += 1;
                current = &mut c.next;
            }
            None
        }

        pub fn get_node(&self, index: usize) -> Option<&'a ListNodeMut<'a, T>> {
            let mut i = 0;
            let mut current = self.clone();
            while let Self::Node(n) = current {
                if i == index {
                    return Some(n);
                }
                i += 1;
                current = n.next.clone()
            }
            None
        }
        pub fn get_node_mut(&mut self, index: usize) -> Option<&'a mut ListNodeMut<'a, T>> {
            let mut i = 0;
            let mut current = self.clone();
            while let Self::Node(n) = current {
                if i == index {
                    return Some(n);
                }
                i += 1;
                current = n.next.clone()
            }
            None
        }

        pub fn reverse(&'a self) -> &'a Self {
            let mut base: &'a ListMut<'_, _> =
                self.get_arena().alloc(ListMut::Empty(self.get_arena()));
            let mut n = self;
            while let ListMut::Node(node) = n {
                base = base.cons(node.value.clone());
                n = &node.next;
            }
            base
        }

        pub fn as_const(&'a self) -> List<'a, T> {
            match self {
                ListMut::Empty(ar) => List::Empty(ar),
                ListMut::Node(n) => {
                    let ar = n.arena;
                    let next = &n.next;
                    let value: &'a T = n.value;
                    let nxt = next.as_const();
                    let node: ListNode<'a, T> = ListNode {
                        value,
                        next: nxt,
                        arena: ar,
                    };
                    let node_ptr = ar.alloc(node);
                    List::Node(node_ptr)
                }
            }
        }

        pub const fn len(&self) -> usize {
            let mut out = 0;
            let mut next = self;
            while let Self::Node(n) = next {
                out += 1;
                next = &n.next;
            }
            out
        }

        pub const fn is_empty(&self) -> bool {
            self.len() == 0
        }
    }
    impl<'a, T: Debug + TrivialClone> Debug for ListMut<'a, T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut dbg = f.debug_list();
            for i in 0..self.len() {
                dbg.entry(&self[i]);
            }
            dbg.finish()
        }
    }

    impl<'a, T: TrivialClone> Iterator for ListMut<'a, T> {
        type Item = T;

        fn next(&mut self) -> Option<Self::Item> {
            match self {
                Self::Empty(_) => None,
                Self::Node(list_node) => {
                    let out = Some(list_node.value.clone());
                    *self = list_node.next.clone();
                    out
                }
            }
        }
    }

    impl<'a, T: TrivialClone> Index<usize> for ListMut<'a, T> {
        type Output = T;
        fn index(&self, index: usize) -> &Self::Output {
            let a: Option<&T> = self.get(index);
            a.unwrap()
        }
    }
    impl<'a, T: TrivialClone> IndexMut<usize> for ListMut<'a, T> {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            self.get_mut(index).unwrap()
        }
    }

    pub struct Map<'a, T: TrivialClone + Hash + Eq, U: TrivialClone> {
        table: &'a mut ListMut<'a, ListMut<'a, (T, U)>>,
    }
    impl<'a, T: TrivialClone + Hash + Eq, U: TrivialClone> Clone for Map<'a, T, U> {
        fn clone(&self) -> Self {
            Self {
                table: self.table.get_arena().alloc(self.table.clone()),
            }
        }
    }
    impl<'a, T: TrivialClone + Hash + Eq, U: TrivialClone> Trivial for Map<'a, T, U> {}
    impl<'a, T: TrivialClone + Hash + Eq, U: TrivialClone> Map<'a, T, U> {
        pub fn new(arena: &'a Arena) -> Self {
            assert!(Self::IS_TRIVIAL);
            let mut list = arena.alloc(ListMut::Empty(arena));
            for _ in 0..64 {
                list = list.cons(ListMut::Empty(arena));
            }
            Self { table: list }
        }
        pub fn with_capacity(arena: &'a Arena, capacity: usize) -> Self {
            assert!(Self::IS_TRIVIAL);
            let mut list = arena.alloc(ListMut::Empty(arena));
            for _ in 0..capacity {
                list = list.cons(ListMut::Empty(arena));
            }
            Self { table: list }
        }

        pub fn insert(&mut self, key: T, value: U) -> Option<U> {
            if self.occupancy() > 1.5 {
                self.resize(self.table.len() * 2);
            }
            let mut hs = DefaultHasher::new();
            key.hash(&mut hs);
            let idx = hs.finish() as usize;
            let len = self.table.len();
            let ls = &mut self.table[idx % len];
            let ls_len = ls.len();
            for i in 0..ls_len {
                let (k, v) = &mut ls[i];
                if *k == key {
                    let mut vp = value;
                    std::mem::swap(v, &mut vp);
                    return Some(vp);
                }
            }
            let ar = ls.get_arena();
            let mut nxt = ListMut::Empty(ar);
            std::mem::swap(&mut nxt, ls);
            let node = ar.alloc(ListNodeMut {
                value: ar.alloc((key, value)),
                next: nxt,
                arena: ar,
            });
            let tmp = ListMut::Node(node);
            self.table[idx % len] = tmp;
            None
        }

        pub fn get<V: PartialEq + Hash>(&self, key: &V) -> Option<&U>
        where
            T: Borrow<V>,
        {
            let mut hs = DefaultHasher::new();
            key.hash(&mut hs);
            let idx = hs.finish() as usize;
            let len = self.table.len();
            let ls = &self.table[idx % len];
            let ls_len = ls.len();
            for i in 0..ls_len {
                let (k, v) = &ls[i];
                if k.borrow() == key {
                    return Some(v);
                }
            }
            None
        }

        pub fn contains<V: PartialEq + Hash>(&self, key: &V) -> bool
        where
            T: Borrow<V>,
        {
            self.get(key).is_some()
        }
        pub fn get_mut<V: PartialEq + Hash>(&mut self, key: &V) -> Option<&mut U>
        where
            T: AsRef<V>,
        {
            let mut hs = DefaultHasher::new();
            key.hash(&mut hs);
            let idx = hs.finish() as usize;
            let len = self.table.len();
            let ls = &mut self.table[idx % len];
            let ls_len = ls.len();
            for i in 0..ls_len {
                let (k, _) = &ls[i];
                if k.as_ref() == key {
                    let (_, v) = &mut ls[i];
                    return Some(v);
                }
            }
            None
        }

        pub fn remove<V: PartialEq + Hash>(&mut self, key: &V) -> Option<(T, U)>
        where
            T: Borrow<V>,
        {
            let mut hs = DefaultHasher::new();
            key.hash(&mut hs);
            let idx = hs.finish() as usize;
            let len = self.table.len();
            let ls = &mut self.table[idx % len];
            let ls_len = ls.len();
            let arena = ls.get_arena();
            for i in 0..ls_len {
                let (k, _) = ls.get(i).unwrap();
                if k.borrow() != key {
                    continue;
                }
                if i == 0 {
                    let nxt = ls.get_node_mut(0).unwrap();
                    let value = nxt.value.clone();
                    let mut nxt_ptr = ListMut::Empty(arena);
                    std::mem::swap(&mut nxt.next, &mut nxt_ptr);
                    self.table[idx % len] = nxt_ptr;
                    return Some(value);
                } else {
                    let nxt = ls.get_node_mut(i).unwrap();
                    let value = nxt.value.clone();
                    let mut nxt_ptr = ListMut::Empty(arena);
                    std::mem::swap(&mut nxt.next, &mut nxt_ptr);
                    ls.get_node_mut(i - 1).unwrap().next = nxt_ptr;
                    return Some(value);
                }
            }
            None
        }

        pub fn resize(&mut self, new_size: usize) {
            let mut out = Self::with_capacity(self.table.get_arena(), new_size);
            for i in 0..self.table.len() {
                for j in 0..self.table[i].len() {
                    let (k, v) = self.table[i][j].clone();
                    out.insert(k, v);
                }
            }
            *self = out;
        }

        pub fn occupancy(&self) -> f64 {
            let len = self.table.len();
            let bins = len as f64;
            let mut hits = 0.0;
            for i in 0..len {
                hits += self.table[i].len() as f64;
            }
            hits / bins
        }

        pub fn get_iter(&self) -> impl Iterator<Item = &(T, U)> {
            struct Out<'a, 'b, T: TrivialClone + Hash + Eq, U: TrivialClone> {
                ix: &'b Map<'a, T, U>,
                i: usize,
                j: usize,
            }

            impl<'a, 'b, T: TrivialClone + Hash + Eq, U: TrivialClone> Iterator for Out<'a, 'b, T, U> {
                type Item = &'b (T, U);
                fn next(&mut self) -> Option<Self::Item> {
                    if self.ix.table.len() <= self.i {
                        return None;
                    }
                    if self.ix.table[self.i].len() <= self.j {
                        self.j = 0;
                        self.i += 1;
                    }
                    if self.ix.table.len() <= self.i {
                        return None;
                    }
                    let out = self.ix.table.get(self.i).unwrap().get(self.j);
                    self.j += 1;
                    out
                }
            }
            Out {
                ix: self,
                i: 0,
                j: 0,
            }
        }
        pub fn get_iter_mut(&'a mut self) -> impl Iterator<Item = &'a mut (T, U)> {
            struct Out<'a, T: TrivialClone + Hash + Eq, U: TrivialClone> {
                ix: &'a mut Map<'a, T, U>,
                i: usize,
                j: usize,
            }

            impl<'a, T: TrivialClone + Hash + Eq, U: TrivialClone> Iterator for Out<'a, T, U> {
                type Item = &'a mut (T, U);
                fn next(&mut self) -> Option<Self::Item> {
                    if self.ix.table.len() <= self.i {
                        return None;
                    }
                    if self.ix.table[self.i].len() <= self.j {
                        self.j = 0;
                        self.i += 1;
                    }
                    if self.ix.table.len() <= self.i {
                        return None;
                    }
                    let node = self.ix.table[self.i].get_node_mut(self.j)?;
                    Some(node.value)
                }
            }
            Out {
                ix: self,
                i: 0,
                j: 0,
            }
        }
    }

    impl<'a, T: TrivialClone + Hash + Eq + Debug, U: TrivialClone + Hash + Eq + Debug> Debug
        for Map<'a, T, U>
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let mut list = f.debug_list();
            for i in 0..self.table.len() {
                for j in 0..self.table[i].len() {
                    let t = self.table[i][j].clone();
                    list.entry(&t);
                }
            }
            list.finish()
        }
    }

    impl<'a, T: TrivialClone + Hash + Eq + Debug, U: TrivialClone + Debug> Display for Map<'a, T, U> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let mut list = f.debug_list();
            for i in 0..self.table.len() {
                for j in 0..self.table[i].len() {
                    let (x, _) = self.table[i][j].clone();
                    list.entry(&x);
                }
            }
            list.finish()
        }
    }

    #[derive(Clone)]
    pub struct Set<'a, T: TrivialClone + Hash + Eq + Debug> {
        internal: Map<'a, T, ()>,
    }
    impl<'a, T: TrivialClone + Hash + Eq + Debug> Trivial for Set<'a, T> {}
    impl<'a, T: TrivialClone + Hash + Eq + Debug> Set<'a, T> {
        pub fn new(arena: &'a Arena) -> Self {
            Self {
                internal: Map::new(arena),
            }
        }

        pub fn with_capacity(arena: &'a Arena, capacity: usize) -> Self {
            Self {
                internal: Map::with_capacity(arena, capacity),
            }
        }

        pub fn insert(&mut self, key: T) {
            self.internal.insert(key, ());
        }

        pub fn contains<V: PartialEq + Hash>(&self, key: &V) -> bool
        where
            T: Borrow<V>,
        {
            self.internal.contains(key)
        }

        pub fn remove<V: PartialEq + Hash>(&mut self, key: &V) -> Option<T>
        where
            T: Borrow<V>,
        {
            self.internal.remove(key).map(|(i, _)| i)
        }

        pub fn resize(&mut self, new_size: usize) {
            self.internal.resize(new_size);
        }

        pub fn occupancy(&self) -> f64 {
            self.internal.occupancy()
        }

        pub fn get_iter(&self) -> impl Iterator<Item = &T> {
            self.internal.get_iter().map(|(i, _)| i)
        }

        pub fn get_iter_mut(&'a mut self) -> impl Iterator<Item = &'a mut T> {
            self.internal.get_iter_mut().map(|(i, _)| i)
        }
    }
    impl<'a, T: TrivialClone + Hash + Eq + Debug> Debug for Set<'a, T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let mut list = f.debug_list();
            for i in 0..self.internal.table.len() {
                for j in 0..self.internal.table[i].len() {
                    let (x, _) = self.internal.table[i][j].clone();
                    list.entry(&x);
                }
            }
            list.finish()
        }
    }

    impl<'a, T: TrivialClone + Hash + Eq + Debug> Display for Set<'a, T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let mut list = f.debug_list();
            for i in 0..self.internal.table.len() {
                for j in 0..self.internal.table[i].len() {
                    let (x, _) = self.internal.table[i][j].clone();
                    list.entry(&x);
                }
            }
            list.finish()
        }
    }

    pub struct BString<'a> {
        buf: &'a mut [u8],
        len: usize,
        arena: &'a Arena,
    }
    impl<'a> Trivial for BString<'a> {}

    impl<'a> BString<'a> {
        pub fn new(arena: &'a Arena) -> Self {
            Self {
                buf: arena.alloc_bytes(16, 1),
                len: 0,
                arena: arena,
            }
        }

        pub fn push(&mut self, ch: char) {
            let sz = ch.len_utf8();
            if self.len + sz < self.buf.len() {
                ch.encode_utf8(&mut self.buf[self.len..self.len + sz]);
            } else {
                let buf2 = self.arena.alloc_bytes(self.buf.len() * 2, 1);
                for i in 0..self.len {
                    buf2[i] = self.buf[i];
                }
                self.buf = buf2;
                ch.encode_utf8(&mut self.buf[self.len..self.len + sz]);
            }
            self.len += sz;
        }

        pub fn get_str(&self) -> &str {
            let bytes = &self.buf[0..self.len];
            std::str::from_utf8(bytes).unwrap()
        }

        pub fn concat(&mut self, v: &str) {
            for i in v.chars() {
                self.push(i);
            }
        }

        pub fn concat_writeable<T: Display>(&mut self, v: &T) {
            {
                std::fmt::write(self, format_args!("{}", v)).unwrap();
            }
        }

        pub fn concat_debug<T: Debug>(&mut self, v: &T) {
            std::fmt::write(self, format_args!("{:#?}", v)).unwrap();
        }

        pub fn take(self) -> &'a str {
            std::str::from_utf8(&self.buf[0..self.buf.len()]).unwrap()
        }
    }

    impl<'a> AsRef<str> for BString<'a> {
        fn as_ref(&self) -> &str {
            self.get_str()
        }
    }

    impl<'a> Display for BString<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.get_str())
        }
    }
    impl<'a> Debug for BString<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.get_str())
        }
    }

    impl<'a> std::fmt::Write for BString<'a> {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            for i in s.chars() {
                self.push(i);
            }
            std::fmt::Result::Ok(())
        }
    }

    impl<'a> Hash for BString<'a> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            let s = self.as_ref();
            s.hash(state);
        }
    }

    pub fn dyn_sprintf<'a>(arena: &'a Arena, format: &str, args: &[&dyn Display]) -> BString<'a> {
        let mut out = BString::new(arena);
        let mut it = format.chars();
        let mut index = 0;
        loop {
            let Some(c) = it.next() else {
                break;
            };
            if c == '%' {
                let Some(c1) = it.next() else {
                    break;
                };
                if c1 == '%' {
                    out.push('%');
                } else if c1 == 'd' {
                    out.concat_writeable(&args[index]);
                    index += 1;
                } else if c1 == 'f' {
                    out.concat_writeable(&args[index]);
                    index += 1;
                } else if c1 == 's' {
                    out.concat_writeable(&args[index]);
                    index += 1;
                } else if c1 == 'u' {
                    out.concat_writeable(&args[index]);
                    index += 1;
                } else if c1 == '*' {
                    out.concat_writeable(&args[index]);
                    index += 1;
                } else {
                    todo!()
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[macro_export]
    macro_rules! sprintf {
    ($arena:expr, $fmt:literal) => {
        dyn_sprintf($arena, $fmt, &[])
    };
    ($arena:expr,$fmt:literal, $($args:expr),+) => {
        dyn_sprintf($arena, $fmt,(&[$(&$args), +]))
    };
}

    pub struct Ptr<'a, T: TrivialClone> {
        ptr: &'a SpinLock<T>,
    }
    impl<'a, T: TrivialClone> Clone for Ptr<'a, T> {
        fn clone(&self) -> Self {
            Self { ptr: self.ptr }
        }
    }
    impl<'a, T: TrivialClone> Copy for Ptr<'a, T> {}
    impl<'a, T: TrivialClone> Ptr<'a, T> {
        pub fn create(arena: &'a Arena, value: T) -> Self {
            Self {
                ptr: arena.alloc(SpinLock::new(value)),
            }
        }

        pub fn load(&self) -> T {
            self.ptr.get()
        }

        pub fn store(&self, value: T) {
            self.ptr.store(value);
        }

        pub fn lock(&self) -> Lock<'a, T> {
            self.ptr.lock()
        }
    }

    impl<'a, T: TrivialClone> Trivial for Ptr<'a, T> {}

    struct SharedListInner<T> {
        mutated: bool,
        locked: bool,
        list: Vec<T>,
    }

    pub struct SharedList<T: Clone> {
        list: Arc<RwLock<SharedListInner<T>>>,
        has_lock: AtomicBool,
    }
    impl<T: Clone> Clone for SharedList<T> {
        fn clone(&self) -> Self {
            let has_lock = AtomicBool::new(self.has_lock.load(std::sync::atomic::Ordering::SeqCst));
            Self {
                list: self.list.clone(),
                has_lock,
            }
        }
    }
    impl<T: Clone> Default for SharedList<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T: Clone> SharedList<T> {
        pub fn new() -> Self {
            Self {
                list: Arc::new(RwLock::new(SharedListInner {
                    locked: false,
                    mutated: false,
                    list: Vec::new(),
                })),
                has_lock: AtomicBool::new(false),
            }
        }
        pub fn handle_locks(&self) {
            let has_lock = self.has_lock.load(std::sync::atomic::Ordering::SeqCst);
            if has_lock {
                return;
            }
            loop {
                let lck = self.list.read().unwrap();
                if !lck.locked {
                    break;
                }
                drop(lck);
                yield_now();
            }
        }
        pub fn push(&self, v: T) {
            self.handle_locks();
            let mut list = self.list.write().unwrap();
            list.list.push(v);
            list.mutated = true;
        }

        pub fn pop(&self) -> Option<T> {
            self.handle_locks();
            let mut list = self.list.write().unwrap();
            list.mutated = true;
            list.list.pop()
        }

        pub fn len(&self) -> usize {
            let list = self.list.read().unwrap();
            list.list.len()
        }

        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }
        pub fn consume_mutation(&self) -> bool {
            self.handle_locks();
            let mut list = self.list.write().unwrap();
            if list.mutated {
                list.mutated = false;
                true
            } else {
                false
            }
        }

        pub fn get(&self, index: usize) -> Option<T> {
            let list = self.list.read().unwrap();
            list.list.get(index).cloned()
        }

        pub fn insert(&self, index: usize, value: T) -> Option<T> {
            self.handle_locks();
            let mut list = self.list.write().unwrap();
            if list.list.len() <= index {
                Some(value)
            } else {
                list.mutated = true;
                list.list.insert(index, value);
                None
            }
        }

        pub fn replace(&self, index: usize, value: T) -> Result<T, T> {
            self.handle_locks();
            let mut list = self.list.write().unwrap();
            let mut v = value;
            if list.list.len() <= index {
                Err(v)
            } else {
                std::mem::swap(&mut v, &mut list.list[index]);
                list.mutated = true;
                Ok(v)
            }
        }

        pub fn set(&self, index: usize, value: T) -> Option<T> {
            self.replace(index, value).err()
        }

        pub fn remove(&self, index: usize) -> Option<T> {
            self.handle_locks();
            let mut list = self.list.write().unwrap();
            if list.list.len() <= index {
                None
            } else {
                list.mutated = true;
                Some(list.list.remove(index))
            }
        }

        pub fn lock(&self) {
            loop {
                let mut list = self.list.write().unwrap();
                if list.locked {
                    drop(list);
                    yield_now();
                } else {
                    self.has_lock
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    list.locked = false;
                    break;
                }
            }
        }

        pub fn has_lock(&self) -> bool {
            self.has_lock.load(std::sync::atomic::Ordering::SeqCst)
        }

        pub fn unlock(&self) {
            if !self.has_lock() {
                return;
            }
            let mut load = self.list.write().unwrap();
            load.locked = false;
            self.has_lock
                .store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }
}
pub mod marathon {
    use super::rtils_useful::{
        BPipe, Exception, Throws, stream_read_bytes_async, stream_read_bytes_blocking,
        stream_try_read_bytes, stream_write_bytes,
    };
    use serde::{Deserialize, Serialize, de::DeserializeOwned};
    use std::{
        collections::{BTreeMap, BTreeSet, VecDeque},
        net::TcpStream,
        sync::{Arc, Mutex},
        thread::yield_now,
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

                    Ok(Some(x))
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

        #[allow(clippy::await_holding_lock)]
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

    /*
        Thing you may want to respond to
    */
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
    #[repr(transparent)]
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
                            Some(out) => std::task::Poll::Ready(Ok(out)),
                            None => std::task::Poll::Pending,
                        },
                        Err(e) => std::task::Poll::Ready(Err(e)),
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
            if let std::collections::btree_map::Entry::Vacant(e) = map.entry(id) {
                e.insert(value);
                return id;
            }
        }
        panic!("too many keys");
    }
    pub fn map_store_high_priority<T: ArachneId, U>(map: &mut BTreeMap<T, U>, value: U) -> T {
        let mut id;
        for i in 1..u64::MAX {
            id = T::create(i);
            if let std::collections::btree_map::Entry::Vacant(e) = map.entry(id) {
                e.insert(value);
                return id;
            }
        }
        panic!("too many keys");
    }

    pub fn map_remove<T: ArachneId, U>(map: &mut BTreeMap<T, U>, id: T) -> Option<U> {
        map.remove(&id)
    }

    pub fn map_copy<T: ArachneId, U: Clone>(map: &BTreeMap<T, U>, id: T) -> Option<U> {
        map.get(&id).cloned()
    }

    pub fn map_get<T: ArachneId, U>(map: &BTreeMap<T, U>, id: T) -> Option<&U> {
        map.get(&id)
    }

    pub fn map_get_mut<T: ArachneId, U>(map: &mut BTreeMap<T, U>, id: T) -> Option<&mut U> {
        map.get_mut(&id)
    }
}
