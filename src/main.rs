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
    use reqwest::Url;
    use std::time::Instant;

    let url1 = Url::parse("https://crawler-test.com").unwrap();
    let url2 = Url::parse("http://crawler-test.com/").unwrap();
    let t = 100000000;

    let start_time = Instant::now();
    for _ in 0..t {
        let _ = url1 == url2;
    }
    println!(
        "Time Taken: {} milli seconds",
        start_time.elapsed().as_millis()
    );

    let url1 = "https://crawler-test.com".to_string();
    let url2 = "http://crawler-test.com/".to_string();

    let start_time = Instant::now();
    for _ in 0..t {
        let _ = url1 == url2;
    }
    println!(
        "Time Taken: {} milli seconds",
        start_time.elapsed().as_millis()
    );
}
