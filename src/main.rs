pub mod state;
pub mod imaglib;
pub mod client;
#[tokio::main]
async fn main(){
    client::Client::run().await;
}