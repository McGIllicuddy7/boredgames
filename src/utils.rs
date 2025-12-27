use std::{
    error::Error,
    fmt::Debug,
    io::{Read, Write},
    net::TcpStream,
};

use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! throws {
    ($t:ty) => {
        Result<$t, Box<dyn std::error::Error>>
    };
    ()=>{
        Result<(), Box<dyn std::error::Error>>
    }
}
#[macro_export]
macro_rules! throw {
    ($e:expr) => {
        return Err(Box::new(Exception::new($e)))
    };
}
#[macro_export]
macro_rules! try_catch {
    ($to_try:block catch |$exp:ident|  $catch:block) => {
        if let Err($exp) = ((|| {$to_try Ok::<(), Box<dyn std::error::Error>>(())}))() $catch
    };
    (($to_try:expr) catch|$exp:ident|  $catch:block) => {
        if let Err($exp) = (|| $to_try)() $catch
    };
}
pub struct Exception<T: std::fmt::Debug> {
    trace: std::backtrace::Backtrace,
    internal: T,
}
impl<T: Debug> std::fmt::Display for Exception<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "in:{:#?} threw exception:{:#?}",
            self.trace, self.internal
        )
    }
}
impl<T: Debug> Exception<T> {
    pub fn new(v: T) -> Self {
        Self {
            trace: std::backtrace::Backtrace::capture(),
            internal: v,
        }
    }
}
impl<T: Debug> Debug for Exception<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "threw exception:{:#?}\n in {:#?}",
            self.internal, self.trace
        )
    }
}
impl<T: Debug> std::error::Error for Exception<T> {}
pub trait AsErr<T> {
    fn as_err(self) -> throws!(T);
}
impl<T> AsErr<T> for Option<T> {
    fn as_err(self) -> crate::throws!(T) {
        match self {
            Some(v) => Ok(v),
            None => {
                throw!("option was none");
            }
        }
    }
}
impl<T, U: Error + 'static> AsErr<T> for Result<T, U> {
    fn as_err(self) -> crate::throws!(T) {
        match self {
            Ok(v) => Ok(v),
            Err(v) => Err(Box::new(v)),
        }
    }
}

pub fn throws(x: i32) -> throws!(i32) {
    throw!(x);
}
#[test]
pub fn test0() -> throws!() {
    try_catch!({throws(10)?;} catch |e| {eprintln!("{:#?}", e);});
    Ok(())
}

pub fn read_object<'a, T: Deserialize<'a>>(
    stream: &mut TcpStream,
    buffer: &'a mut Vec<u8>,
) -> throws!(T) {
    let _=  stream.set_nonblocking(false);
    let mut buff = [0; 8];
    stream.read_exact(&mut buff)?;
    let size = u64::from_le_bytes(buff);
    buffer.clear();
    for _ in 0..size as usize {
        buffer.push(0);
    }
    stream.read_exact(buffer)?;
    Ok(serde_json::de::from_slice(buffer)?)
}

pub fn try_read_object<'a, T: Deserialize<'a>>(
    stream: &mut TcpStream,
    buffer: &'a mut Vec<u8>,
) -> throws!(Option<T>) {
    stream.set_nonblocking(true)?;
    let mut buff = [0; 8];
    if let Err(e) = stream.read_exact(&mut buff) {
        stream.set_nonblocking(false)?;
        if e.kind() == std::io::ErrorKind::WouldBlock {
            return Ok(None);
        }
        return Err(Box::new(e));
    }
    let size = u64::from_le_bytes(buff);
    buffer.clear();
    for _ in 0..size as usize {
        buffer.push(0);
    }
    stream.set_nonblocking(false)?;
    if let Err(e) = stream.read_exact(buffer) {
        stream.set_nonblocking(false)?;
        if e.kind() == std::io::ErrorKind::WouldBlock {
            return Ok(None);
        }
        return Err(Box::new(e));
    }
    stream.set_nonblocking(false)?;
    Ok(Some(serde_json::de::from_slice(buffer)?))
}

pub fn write_object<T: Serialize>(stream: &mut TcpStream, v: &T) -> throws!() {
   let _=  stream.set_nonblocking(false);
    let s = serde_json::to_string(v)?;
    let size: [u8; 8] = u64::to_ne_bytes((s.len() as u64).to_le());
    stream.write(&size)?;
    stream.write(s.as_bytes())?;
    Ok(())
}
