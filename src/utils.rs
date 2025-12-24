use std::{error::Error, fmt::Debug};


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
        if let Err($exp) = (|| $to_try)() $catch
    };
    (($to_try:expr) catch|$exp:ident|  $catch:block) => {
        if let Err($exp) = (|| $to_try)() $catch
    };
}
pub struct Exception<T:std::fmt::Debug>{
    trace:std::backtrace::Backtrace,
    internal:T
}
impl<T:Debug> std::fmt::Display for Exception<T>{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "in:{:#?} threw exception:{:#?}", self.trace, self.internal)
    }
}
impl<T:Debug> Exception<T>{
    pub fn new(v:T)->Self{
        Self { trace: std::backtrace::Backtrace::capture(), internal: v }
    }
}
impl<T:Debug> Debug for Exception<T>{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "threw exception:{:#?}\n in {:#?}",self.internal, self.trace)
    }
}
impl<T:Debug> std::error::Error for Exception<T>{

}
pub trait AsErr<T>{
    fn as_err(self)->throws!(T);
}
impl<T> AsErr<T> for Option<T>{
    fn as_err(self)->crate::throws!(T) {
        match self{
            Some(v) =>{ return Ok(v);},
            None => {throw!("option was none");},
        }
    }
}
impl<T, U:Error+'static> AsErr<T> for Result<T, U>{
    fn as_err(self)->crate::throws!(T) {
        match self{
            Ok(v) =>{ return Ok(v);},
            Err(v) => {return Err(Box::new(v));},
        }
    }
}

pub fn throws(x:i32)->throws!(i32){
    throw!(x);
}
#[test]
pub fn test0()->throws!(){
    try_catch!({throws(10)} catch |e| {eprintln!("{:#?}", e);});
    Ok(())
}