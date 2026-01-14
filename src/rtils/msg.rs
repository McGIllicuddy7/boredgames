use std::{collections::HashMap, sync::{Arc, LazyLock}};

use serde::{Deserialize, Serialize};

use crate::{DEFINE_ID_WRAPPER, events::IDS, Throws, events::{ThreadSafeIsh, EventSync}};
use concat_idents::concat_idents;

DEFINE_ID_WRAPPER!(ObjectId);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message{
    pub target_id:ObjectId,
    pub to_call:Arc<str>,
    pub message:Arc<[String]>
}
pub trait Object:ThreadSafeIsh+{
    fn call(&mut self, message:Message);
    fn can_accept(&self, message:Message)->bool;
}
#[macro_export]
macro_rules! des_arg {
    ($x:ty,$args:expr) => {
        serde_json::from_str($args.next().unwrap()).unwrap()
    };
}

#[macro_export]
macro_rules! make_callable_function{
    ($lower_case_name:ident,$name:ident,  $( $x:ty),*) => {
        concat_idents!(rname = $lower_case_name, _, $name, _ ,wrapper {
            pub fn rname(ptr:&mut Self,args:Vec<String>){
                let mut args = args.iter();
                ptr.$name(  
                    $(
                        des_arg!($x, args),
                    )*
                );
                assert!(args.next().is_none());
            }
        });
    };
}

#[macro_export]
macro_rules! make_method{
    ($lower_case_name:ident,$self_name:ident, $name:ident,$( $y:ident:$x:ty),*) => {
        concat_idents!(rname = $lower_case_name, _, $name, _ ,wrapper{
            pub fn rname(object:ObjectId,events:EventSync<impl ThreadSafeIsh>,$( $y:$x),*)->Throws<()>{
                let mut args = Vec::new();
                $(
                    args.push(serde_json::to_string_pretty(&$y)?);
                )*
                let name = stringify!($name);
                let msg = Message{
                    message:args.into(), 
                    to_call:name.into(), 
                    target_id:object
                };
                events.new_message(msg)?;
                Ok(())
        }
        });
        concat_idents!(direct_wrapper_name = $lower_case_name, _, $name, _ ,wrapper,_direct {
            pub fn direct_wrapper_name(ptr:&mut $self_name, $( $y:$x),*)->Throws<()>{
                let mut args = Vec::new();
                $(
                    args.push(serde_json::to_string_pretty(&$y)?);
                )*
                let name = stringify!($name);
                let msg = Message{
                    message:args.into(), 
                    to_call:name.into(), 
                    target_id:ObjectId::invalid(),
                };
                ptr.call(msg);
                Ok(())
            }
        });
    };
}

#[macro_export]
macro_rules! method_table {
    ($lower_case_name:ident,$self_name:ident,($($name:ident),*)) => {
        pub fn create_method_table()->std::collections::HashMap<String,fn(&mut $self_name, Vec<String>)>{
            let mut out = std::collections::HashMap::new();
            $(
                concat_idents!(wrapper_name = $lower_case_name, _, $name, _ ,wrapper{
                out.insert(stringify!($name).to_string(), Self::wrapper_name as fn(&mut $self_name, Vec<String>));
                });
            )*
            out
        }
    };
}


#[macro_export]
macro_rules! define_method{
    ($self_name:ident,$lower_case_name:ident,$((fn $name:ident ($($y:ident:$x:ty),*))),*) => {
        impl $self_name{
            $(
                make_callable_function!($lower_case_name,$name, $($x),*);
            )*
            method_table!(
                $lower_case_name,
                $self_name, ($(
                   $name
                ),*)
            );
        }
        $(
            make_method!($lower_case_name,$self_name, $name, $($y:$x),*);
        )*
        impl Object for $self_name{
            fn call(&mut self, message:Message) {
                static TABLE:LazyLock<HashMap<String,fn(&mut $self_name, args:Vec<String>)>> = LazyLock::new(||{$self_name::create_method_table()});
                (TABLE.get(message.to_call.as_ref()).unwrap())(self,message.message.clone().to_vec())
            }
            fn can_accept(&self, message:Message)->bool{
                static TABLE:LazyLock<HashMap<String,fn(&mut $self_name, args:Vec<String>)>> = LazyLock::new(||{$self_name::create_method_table()});
                (TABLE.get(message.to_call.as_ref()).is_some())
            }
        }
    };
}

pub struct TestObject{

}
impl TestObject{
    pub fn test(&mut self,x:i32, y:i32){
        println!("x:{}", x+y);
    }
    pub fn test_2(&mut self, x1:i32){
        println!("x1:{}",x1*2);
    }
}

define_method!(TestObject,
    test_object,
    (fn test(x:i32, y:i32)),
    (fn test_2(x1:i32))
);