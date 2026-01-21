use raylib::{color::Color, prelude::RaylibDraw};
use transir::trans;

use crate::{sharedlist::{SharedList, SpinRwLock}, transgui::{ElementId, TransGui, TransIr}};
#[allow(unused)]
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::yield_now;
pub mod rtils;
pub mod server;
pub mod sharedlist;
pub mod state;
pub mod tgui;
pub mod transgui;
pub fn old_main() {
    let indexes = SharedList::new();
    let indices2 = indexes.clone();
    let indices3 = indexes.clone();
    let x = trans!(
            (div (
                (text "hello world")
                (button "create element" ({
                    let on_click = move |x:&mut TransGui, id:ElementId|{
                        indices2.push(format!("index:{:#?}", indices2.len()));
                    //    println!("{:#?}", id);
                    } ;
                    on_click
                }))
                (button "destroy element" ({
                    let on_click = move |x:&mut TransGui, id:ElementId|{
                        indices3.pop_front();
                    //    println!("{:#?}", id);
                    } ;
                    on_click
                }))
                (scrollbox name = "bridget" w 10 h 15(
                ))
            )
        )

    );
    let mut gui = TransIr::to_gui(x);
    let (mut handle, thread) = raylib::init().size(0, 0).title("testing").build();
    let w = handle.get_screen_height();
    handle.set_window_size((w * 22) / 16, w);
    handle.set_target_fps(60);
    while !handle.window_should_close() {
        let mut dr = handle.begin_drawing(&thread);
        dr.clear_background(Color::BLACK);
        let id = gui.get_name_id("bridget");
        gui.update(&mut dr);
    }
}

pub fn main() { 
    let list:SharedList<u64> = SharedList::new();
    let mut handles = Vec::new();
    for i in 0..8*10{
        list.push((i) as u64);
    }
    for i in 0..8{
        let listclone= list.clone();

        handles.push(std::thread::spawn(move ||{
            std::thread::yield_now(); 
            let list = listclone;
            let start = i*10;   
             println!("started:{}", start);
            let mut z = list.write_at(start as usize).unwrap();
            let mut should_eq = *z.get();
            for x in 0..10{
                std::thread::yield_now(); 
                for j in 1..10{
                        std::thread::yield_now(); 
                        let get = list.get(start+j).unwrap();
                        should_eq+= get;
                        *z.get_mut() += get;
                    }
            }
            println!("should_eq:{:#?}", should_eq);
            drop(z);
            println!("done with:{}", start);
        }
         ));
    }
    for i in handles{
        i.join().unwrap();
    }
    for i in list{
        println!("{}",i);
    }
}
