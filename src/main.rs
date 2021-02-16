use std::time::Instant;
mod retriever;

#[tokio::main]
async fn main() {
    let now = Instant::now();
    println!("Started");
    retriever::temp().await;
    println!("Time: {}", now.elapsed().as_secs());
}
