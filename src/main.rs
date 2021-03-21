mod cli;
mod crawler;
mod extractors;
mod file_handler;

#[tokio::main]
async fn main() {
    cli::entry().await;
    // test().await;
}

#[cfg(debug_assertions)]
async fn test() {
    use std::path::PathBuf;

    let p = PathBuf::from("/home/ayush/Documents/Programming/Projects/WOC/whitelist.txt");
    println!("{:#?}", file_handler::read_hosts(p).await.unwrap());
}
