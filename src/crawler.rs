use crate::extractors::links;
use futures::future;
use links::Link;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::mpsc;

pub async fn crawl(
    origin_url: Link,
    crawl_depth: Option<usize>,
    whitelist: Option<HashSet<String>>,
    blacklist: Option<HashSet<String>>,
    tx: mpsc::UnboundedSender<Link>,
) {
    let mut to_crawl: HashSet<Link> = HashSet::new();
    let mut crawled: HashSet<Arc<String>> = HashSet::new();
    let mut dont_crawl: HashSet<Arc<String>> = HashSet::new();

    to_crawl.insert(origin_url.clone());

    #[cfg(debug_assertions)]
    let mut count: usize = 1;

    while !to_crawl.is_empty() {
        #[cfg(debug_assertions)]
        {
            println!("Pass: {}, Tasks: {}", count, to_crawl.len());
            count += 1;
        }

        let crawl_handler = to_crawl.iter().map(|x| crawl_page(&x));
        let temp_links = future::join_all(crawl_handler).await;
        let temp_found_links =
            temp_links
                .iter()
                .map(|x| x.1.clone())
                .fold(HashSet::new(), |mut acc, x| {
                    acc.extend(x);
                    acc
                });

        temp_links.iter().for_each(|x| {
            tx.send(x.0.clone());
        });
        crawled.extend(temp_links.iter().map(|x| x.0.url.clone()));

        temp_found_links
            .iter()
            .filter(|x| !x.should_crawl(crawl_depth, &whitelist, &blacklist))
            .filter(|x| !dont_crawl.contains(&x.url))
            .for_each(|x| {
                tx.send(x.clone());
            });

        dont_crawl.extend(
            temp_found_links
                .iter()
                .filter(|x| !x.should_crawl(crawl_depth, &whitelist, &blacklist))
                .map(|x| x.url.clone()),
        );

        to_crawl = temp_found_links
            .iter()
            .filter(|x| x.should_crawl(crawl_depth, &whitelist, &blacklist))
            .filter(|x| !crawled.contains(&x.url))
            .cloned()
            .collect::<HashSet<Link>>();
    }
}

pub async fn crawl_page(link: &Link) -> (Link, HashSet<Link>) {
    let mut link = link.clone();
    let url_temp = link.url.clone();
    let get_page_handler = tokio::spawn(get_page(url_temp)).await;
    let response = if let Ok(Ok(x)) = get_page_handler {
        x
    } else {
        return (link, HashSet::new());
    };

    link.update_from_response(&response);
    if link.check_html() {
        let html = match response.text().await {
            Ok(x) => x,
            Err(_) => return (link, HashSet::new()),
        };
        let url_clone = link.url.clone();
        let links =
            tokio::task::spawn_blocking(move || links::get_links_from_html(&html, url_clone)).await;
        return match links {
            Ok(x) => (link, x),
            Err(_) => (link, HashSet::new()),
        };
    }
    (link, HashSet::new())
}

pub async fn get_page(url: Arc<String>) -> Result<reqwest::Response, reqwest::Error> {
    //! Function to make get request to a single url.
    let resp = reqwest::get(url.as_str()).await?;
    resp.error_for_status()
}
