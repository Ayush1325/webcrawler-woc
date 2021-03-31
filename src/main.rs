mod cli;
mod crawler;
mod extractors;
mod file_handler;

#[tokio::main]
async fn main() {
    cli::entry().await;
    //test().await;
}

#[allow(dead_code)]
async fn test() {
    use thirtyfour::prelude::*;

    let mut caps = DesiredCapabilities::chrome();
    caps.add_chrome_arg("--enable-automation").unwrap();
    let driver = WebDriver::new("http://localhost:4444/wd/hub", &caps)
        .await
        .unwrap();
    let p = std::path::PathBuf::from("/media/storage/test.png");

    driver.get("https://wikipedia.org").await.unwrap();
    driver.fullscreen_window().await.unwrap();
    driver.screenshot(&p).await.unwrap();
}
