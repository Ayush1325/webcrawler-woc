use crate::extractors::links;
use futures::{stream, StreamExt};
use links::Link;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::mpsc;

pub async fn crawl(
    origin_url: Link,
    crawl_depth: Option<usize>,
    whitelist: Option<HashSet<String>>,
    blacklist: Option<HashSet<String>>,
    tx: mpsc::UnboundedSender<Link>,
    task_limit: usize,
) {
    let mut to_crawl: HashSet<Link> = HashSet::new();
    let mut crawled: HashSet<Arc<String>> = HashSet::new();
    let mut dont_crawl: HashSet<Arc<String>> = HashSet::new();
    let client = reqwest::Client::new();

    to_crawl.insert(origin_url.clone());

    #[cfg(debug_assertions)]
    let mut count: usize = 1;

    while !to_crawl.is_empty() {
        #[cfg(debug_assertions)]
        {
            println!("Pass: {}, Tasks: {}", count, to_crawl.len());
            count += 1;
        }

        let mut crawls = stream::iter(to_crawl.clone())
            .map(|x| {
                let c = client.clone();
                tokio::spawn(async move { crawl_page(x, c).await })
            })
            .buffer_unordered(task_limit);

        to_crawl.clear();

        while let Some(x) = crawls.next().await {
            if let Ok((link, found_links)) = x {
                crawled.insert(link.url.clone());
                tx.send(link.clone());

                dont_crawl.extend(
                    found_links
                        .iter()
                        .filter(|x| !x.should_crawl(crawl_depth, &whitelist, &blacklist))
                        .map(|x| x.url.clone()),
                );
                found_links
                    .iter()
                    .filter(|x| !x.should_crawl(crawl_depth, &whitelist, &blacklist))
                    .filter(|x| !dont_crawl.contains(&x.url))
                    .for_each(|x| {
                        tx.send(x.clone());
                    });

                to_crawl.extend(
                    found_links
                        .iter()
                        .filter(|x| x.should_crawl(crawl_depth, &whitelist, &blacklist))
                        .filter(|x| !crawled.contains(&x.url))
                        .cloned()
                        .collect::<HashSet<Link>>(),
                );
            }
        }
    }
}

pub async fn crawl_page(link: Link, client: reqwest::Client) -> (Link, HashSet<Link>) {
    //! TODO: Maybe Return HashSet<String> instead of HashSet<Link>
    let mut link = link.clone();
    let url_temp = link.url.clone();
    let get_page_handler = get_page(url_temp, &client).await;
    let response = match get_page_handler {
        Ok(x) => x,
        Err(_) => return (link, HashSet::new()),
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

pub async fn get_page(
    url: Arc<String>,
    client: &reqwest::Client,
) -> Result<reqwest::Response, reqwest::Error> {
    //! Function to make get request to a single url.
    let resp = client.get(url.as_str()).send().await?;
    resp.error_for_status()
}
