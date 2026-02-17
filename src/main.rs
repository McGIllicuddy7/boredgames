pub mod client;
pub mod dos;
pub mod id;
pub mod rtils;
pub mod state;
pub mod voip;
pub fn testing123() -> i32 {
    24
}
global!(x:i32 = testing123());
pub fn main() {
    println!("{}", x.load());
    x.store(32);
    println!("{}", x.load());
    //setup(main_func);
}
