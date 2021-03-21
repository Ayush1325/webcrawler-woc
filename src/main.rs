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
    use mime::Mime;
    let m = "text/plain".parse::<Mime>().unwrap();
    println!("{}", m == mime::TEXT_PLAIN);
}
