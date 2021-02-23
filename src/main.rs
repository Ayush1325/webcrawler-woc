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

    let url = "https://docs.rs/reqwest/0.11.1/reqwest/struct.Response.html";
    let resp = reqwest::get(url).await.unwrap();
    let mime_test = resp.headers().get(reqwest::header::CONTENT_TYPE).unwrap();
    let parsed: Mime = mime_test.to_str().unwrap().parse().unwrap();
    if parsed.type_() == mime::TEXT {
        println!("{:#?}", parsed);
    }
}
