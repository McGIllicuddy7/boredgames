use std::{
    backtrace,
    fmt::Display,
    str::FromStr,
};


pub mod events;
pub mod server;
pub mod msg;

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

        let f =|| {
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
