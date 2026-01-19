pub mod rtils_useful {
    #[allow(unused)]
    use std::backtrace;
    use std::collections::VecDeque;
    use std::error::Error;
    use std::fmt::Display;

    use std::io::{Read, Write};
    use std::str::FromStr;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
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
                    if let Some(e) = end_delim {
                        if e == '{' || e == '}' {
                            let Some(n) = fmt.next() else {
                                return false;
                            };
                            if n != e {
                                return false;
                            }
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
            } else {
                if i != j {
                    return false;
                }
            }
            if done {
                break;
            }
        }
        if let Some(_) = inp.next() {
            return false;
        }
        if args_index != args.len() {
            return false;
        }
        return true;
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
            return Exception {
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
            return Exception {
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
                Ok(x) => match x {
                    Some(t) => Some(Ok(t)),
                    None => None,
                },
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
        stream.write(&blen)?;
        stream.write(bytes)?;
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
        for _ in 0..len {
            buf.push(0_u8);
        }
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
        for _ in 0..len {
            buf.push(0_u8);
        }
        let e = stream.read_exact(&mut buf);
        stream.set_nonblocking(true)?;
        if let Err(e) = e {
            throw!(e);
        }
        Ok(buf)
    }


    #[repr(transparent)]
    #[derive(Clone, Debug, Copy,Hash, PartialEq, Eq,PartialOrd, Ord, Serialize, Deserialize)]
    pub struct Immutable<T>{
        x:T
    }
    impl<T> Immutable<T>{
        pub fn new(x:T)->Self{
            Self { x }
        }
        pub fn get(&self)->&T{
            &self.x
        }
        pub fn unsafe_get_mut_please_dont_use(&mut self)->&mut T{
            &mut self.x
        }
        pub unsafe fn get_mut(&mut self)->&mut T{
            &mut self.x
        }
        pub fn take(self)->T{
            self.x
        }
    }
}
