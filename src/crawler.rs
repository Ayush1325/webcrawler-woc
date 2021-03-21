use crate::extractors::links;
use futures::{stream, StreamExt};
use links::Link;
use reqwest::Url;
use std::collections::HashSet;
use tokio::sync::mpsc;

pub async fn crawl_with_depth(
    origin_url: Link,
    crawl_depth: usize,
    whitelist: Option<HashSet<String>>,
    blacklist: Option<HashSet<String>>,
    tx_output: mpsc::Sender<Link>,
    task_limit: usize,
) {
    let mut to_crawl: HashSet<Url> = HashSet::new();
    let mut crawled: HashSet<Url> = HashSet::new();
    let mut dont_crawl: HashSet<Url> = HashSet::new();
    let client = reqwest::Client::new();

    to_crawl.insert(origin_url.url);

    for _ in 0..crawl_depth {
        println!("Crawling {} URls", to_crawl.len());

        let (tx_cralwer, mut rx_crawler) = mpsc::channel::<Link>(task_limit);

        stream::iter(to_crawl.clone())
            .for_each(|x| async {
                let tx_clone = tx_cralwer.clone();
                let client_clone = client.clone();
                tokio::spawn(
                    async move { crawl_page(x, client_clone, tx_clone, task_limit).await },
                );
            })
            .await;

        to_crawl.clear();

        drop(tx_cralwer);

        while let Some(link) = rx_crawler.recv().await {
            if link.crawled {
                crawled.insert(link.url.clone());
                if let Err(_) = tx_output.send(link).await {
                    return;
                }
            } else {
                let should_crawl = link.should_crawl(&whitelist, &blacklist);
                if should_crawl && !crawled.contains(&link.url) {
                    to_crawl.insert(link.url);
                } else if !should_crawl && !dont_crawl.contains(&link.url) {
                    dont_crawl.insert(link.url.clone());
                    if let Err(_) = tx_output.send(link).await {
                        return;
                    }
                }
            }
        }
    }

    stream::iter(to_crawl)
        .map(|x| links::Link::from_url(&x))
        .for_each_concurrent(task_limit, |x| async {
            let _ = tx_output.send(x).await;
        })
        .await;
}

pub async fn crawl_no_depth(
    origin_url: Link,
    whitelist: Option<HashSet<String>>,
    blacklist: Option<HashSet<String>>,
    tx_output: mpsc::Sender<Link>,
    task_limit: usize,
) {
    let mut to_crawl: HashSet<Url> = HashSet::new();
    let mut crawled: HashSet<Url> = HashSet::new();
    let mut dont_crawl: HashSet<Url> = HashSet::new();
    let client = reqwest::Client::new();

    to_crawl.insert(origin_url.url.clone());

    let mut first_crawl = true;

    while !to_crawl.is_empty() {
        println!("Crawling {} URls", to_crawl.len());

        let (tx_cralwer, mut rx_crawler) = mpsc::channel::<Link>(task_limit);

        if first_crawl {
            let tx_clone = tx_cralwer.clone();
            let client_clone = client.clone();
            let url = origin_url.url.clone();
            tokio::spawn(async move {
                crawl_sitemaps(url, tx_clone, task_limit, client_clone).await;
            });
            first_crawl = false;
        }

        stream::iter(to_crawl.clone())
            .for_each_concurrent(task_limit, |x| async {
                let tx_clone = tx_cralwer.clone();
                let client_clone = client.clone();
                tokio::spawn(
                    async move { crawl_page(x, client_clone, tx_clone, task_limit).await },
                );
            })
            .await;

        to_crawl.clear();

        drop(tx_cralwer);

        while let Some(link) = rx_crawler.recv().await {
            if link.crawled {
                crawled.insert(link.url.clone());
                if let Err(_) = tx_output.send(link).await {
                    return;
                }
            } else {
                let should_crawl = link.should_crawl(&whitelist, &blacklist);
                if should_crawl && !crawled.contains(&link.url) {
                    to_crawl.insert(link.url);
                } else if !should_crawl && !dont_crawl.contains(&link.url) {
                    dont_crawl.insert(link.url.clone());
                    if let Err(_) = tx_output.send(link).await {
                        return;
                    }
                }
            }
        }
    }
}

pub async fn crawl_page(url: Url, client: reqwest::Client, tx: mpsc::Sender<Link>, limit: usize) {
    let mut link = links::Link::from_url(&url);
    let resp = match get_page(url.as_str(), &client).await {
        Ok(x) => x,
        Err(_) => {
            link.crawled = true;
            let _ = tx.send(link).await;
            return;
        }
    };
    link.update_from_response(&resp);
    let is_html = link.check_mime_from_list(&[mime::TEXT_HTML, mime::TEXT_HTML_UTF_8]);
    if let Err(_) = tx.send(link).await {
        return;
    }
    if is_html {
        let html = match resp.text().await {
            Ok(x) => x,
            Err(_) => return,
        };
        let links = links::get_links_from_html(&html, url.as_str());
        let tx_ref = &tx;
        stream::iter(links)
            .for_each_concurrent(limit, |x| async move {
                let _ = tx_ref.send(x).await;
            })
            .await;
    }
}

pub async fn crawl_sitemaps(
    url: Url,
    tx: mpsc::Sender<Link>,
    limit: usize,
    client: reqwest::Client,
) {
    let mut robottxt_url = url.clone();
    robottxt_url.set_path("robots.txt");
    let robottxt = match get_page(robottxt_url.as_str(), &client).await {
        Ok(x) => match x.text().await {
            Ok(x) => x,
            Err(_) => return,
        },
        Err(_) => return,
    };
    let url_str = url.to_string();
    robottxt
        .lines()
        .filter(|x| x.contains("Sitemap"))
        .filter_map(|x| x[9..].split_whitespace().next())
        .map(|x| x.trim())
        .filter_map(|x| links::normalize_url(x, &url_str))
        .for_each(|x| {
            let tx_clone = tx.clone();
            let client_clone = client.clone();
            let limit_clone = limit.clone();
            tokio::spawn(async move {
                crawl_sitemap(x.url, tx_clone, limit_clone, client_clone).await;
            });
        });
}

async fn crawl_sitemap(url: Url, tx: mpsc::Sender<Link>, limit: usize, client: reqwest::Client) {
    let mut link = links::Link::from_url(&url);
    let resp = match get_page(url.as_str(), &client).await {
        Ok(x) => x,
        Err(_) => return,
    };
    link.update_from_response(&resp);
    let text = match resp.text().await {
        Ok(x) => x,
        Err(_) => return,
    };
    let links = match link.content_type {
        Some(x) => match (x.type_(), x.subtype()) {
            (mime::TEXT, mime::PLAIN) => links::get_links_from_text(&text, url.as_str()),
            _ => return,
        },
        None => return,
    };
    let tx_ref = &tx;
    stream::iter(links)
        .for_each_concurrent(limit, |x| async {
            let _ = tx_ref.send(x).await;
        })
        .await;
}

pub async fn get_page(
    url: &str,
    client: &reqwest::Client,
) -> Result<reqwest::Response, reqwest::Error> {
    let resp = client.get(url).send().await?;
    resp.error_for_status()
}
