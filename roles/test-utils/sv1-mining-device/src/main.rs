pub(crate) mod client;
pub(crate) mod job;
pub(crate) mod miner;
pub use client::Client;

#[async_std::main]
async fn main() {
    Client::connect(80).await
}
