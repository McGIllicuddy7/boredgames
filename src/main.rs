use std::fmt::Debug;

use crate::utils::{Heap, HeapRef, WeakHeap};

pub mod engine;
pub mod libgui;
//https://lib.rs/crates/wrappe
pub mod utils;

fn main() {
    //let (mut handle, thread) = raylib::RaylibBuilder::default()
    //        .resizable()
    //       .width(18 * 1920 / 20)
    //      .height(18 * 1080 / 20)
    //     .build();
    //handle.set_exit_key(None);
    //engine::game_loop(&mut handle, &thread).await;
    let heap: Heap<GraphNode<i32>> = Heap::new();
    let mut values = Vec::new();
    for i in 0..100 {
        let wk = heap.downgrade();
        values.push(heap.alloc_blocking(GraphNode {
            alloc: wk,
            value: i,
            connections: Vec::new(),
        }))
    }
    for i in 0..100 {
        let mut base = values[i].clone();
        let mut g = base.lock_blocking();
        for j in 0..100 {
            if i == j {
                continue;
            }
            g.connections.push(values[j].clone());
        }
    }
    for mut i in values {
        let mut tmp = i.lock_blocking();
        println!("{}[", tmp.value);
        for j in &mut tmp.connections {
            let t2 = j.lock_blocking();
            println!("\t{}", t2.value);
        }
        println!("]");
    }
}

pub struct GraphNode<T: Debug> {
    pub alloc: WeakHeap<GraphNode<T>>,
    pub value: T,
    pub connections: Vec<HeapRef<GraphNode<T>>>,
}
impl<T: Debug> Drop for GraphNode<T> {
    fn drop(&mut self) {
        println!("dropped:{:#?}", self.value);
    }
}
