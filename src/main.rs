use std::time::Instant;
mod crawler;
mod extractors;

#[tokio::main]
async fn main() {
    let now = Instant::now();
    println!("Started");
    temp().await;
    println!("Time: {}", now.elapsed().as_millis());
}

async fn temp() -> () {
    // let url = "https://www.wikipedia.org/";
    // let page = get_page(&url).await.unwrap();
    // let html = page.text().await.unwrap();
    // let links = get_links_from_html(&html, &url);
    // links.iter().for_each(|x| println!("{}", x.as_str()));
    let origin_url = "https://crawler-test.com/".to_string();
    // let local_url = "http://127.0.0.1:5500/index.html".to_string();
    let links = crawler::crawl_host(origin_url, crawler::CrawlDepth::Variable(1))
        .await
        .unwrap();
    links.iter().for_each(|x| println!("{}", x));
    // let url = Url::parse("https://www.wikipedia.org/home.html").unwrap();
    // println!("{}", url.path_segments().unwrap().count());
}
