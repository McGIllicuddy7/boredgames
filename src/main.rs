pub mod rtils;
pub mod server;
pub mod state;
pub mod tuiclient;

fn main() {
    let mut client = tuiclient::TuiClient::new();
    loop {
        println!("what");
        client.run();
    }
}
