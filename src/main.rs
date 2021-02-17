mod cli;
mod crawler;
mod extractors;

#[tokio::main]
async fn main() {
    cli::entry().await;
}
