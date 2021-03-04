use crate::extractors::links;
use futures::{stream, StreamExt};
use links::Link;
use reqwest::Url;
use std::collections::HashSet;
use tokio::sync::mpsc;

pub async fn crawl(
    origin_url: Link,
    crawl_depth: Option<usize>,
    whitelist: Option<HashSet<String>>,
    blacklist: Option<HashSet<String>>,
    tx_output: mpsc::UnboundedSender<Link>,
    task_limit: usize,
) {
    //! Bug: https://crawler-test.com and http://crawler-test.com are being crawled every single time.
    let mut to_crawl: HashSet<Url> = HashSet::new();
    let mut crawled: HashSet<Url> = HashSet::new();
    let mut dont_crawl: HashSet<Url> = HashSet::new();
    let client = reqwest::Client::new();

    to_crawl.insert(origin_url.url);

    while !to_crawl.is_empty() {
        println!("Crawling {} URls", to_crawl.len());

        let (tx_cralwer, mut rx_crawler) = mpsc::channel::<Link>(task_limit);

        stream::iter(to_crawl.clone())
            .for_each_concurrent(task_limit, |x| async {
                let tx_clone = tx_cralwer.clone();
                let client_clone = client.clone();
                tokio::spawn(async move { crawl_page(x, client_clone, tx_clone).await });
            })
            .await;

        to_crawl.clear();

        drop(tx_cralwer);

        while let Some(link) = rx_crawler.recv().await {
            if link.crawled {
                crawled.insert(link.url.clone());
                if let Err(_) = tx_output.send(link) {
                    return;
                }
            } else {
                let should_crawl = link.should_crawl(&crawl_depth, &whitelist, &blacklist);
                if should_crawl && !crawled.contains(&link.url) {
                    to_crawl.insert(link.url);
                } else if !should_crawl && !dont_crawl.contains(&link.url) {
                    dont_crawl.insert(link.url.clone());
                    if let Err(_) = tx_output.send(link) {
                        return;
                    }
                }
            }
        }
    }
}

pub async fn crawl_page(url: Url, client: reqwest::Client, tx: mpsc::Sender<Link>) {
    let resp = match get_page(url.as_str(), &client).await {
        Ok(x) => x,
        Err(_) => return,
    };
    let link = links::Link::from_response(&resp);
    let is_html = link.check_html();
    if let Err(_) = tx.send(link).await {
        return;
    }
    if is_html {
        let html = match resp.text().await {
            Ok(x) => x,
            Err(_) => return,
        };
        links::get_links_from_html(&html, url.as_str(), &tx).await;
    }
}

pub async fn get_page(
    url: &str,
    client: &reqwest::Client,
) -> Result<reqwest::Response, reqwest::Error> {
    let resp = client.get(url).send().await?;
    resp.error_for_status()
}
