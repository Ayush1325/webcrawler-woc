mod cli;
mod crawler;
mod extractors;
mod file_handler;

#[tokio::main]
async fn main() {
    cli::entry().await;
    // test().await;
}

#[allow(dead_code)]
async fn test() {
    use url::Url;

    let t = Url::parse("tel:+6494461709").unwrap();
    println!("{}", t.scheme() == "tel");
}
