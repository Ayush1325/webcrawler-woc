/*!
Module Containing the Crawler functions.
*/
use crate::extractors::links;
use futures::{stream, StreamExt};
use links::Link;
use reqwest::Url;
use std::time::Duration;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::mpsc;

/// Function to initialize Reqwest Client.
/// Also specifies the timeout.
fn init_reqwest_client(timeout: u64) -> Result<reqwest::Client, String> {
    let client_builder = reqwest::ClientBuilder::new().timeout(Duration::new(timeout, 0));
    match client_builder.build() {
        Ok(x) => Ok(x),
        Err(_) => Err("Could not build http client".to_string()),
    }
}

/// Function to initialize DNS resolver.
fn init_dns_resolver() -> Result<trust_dns_resolver::TokioAsyncResolver, String> {
    match trust_dns_resolver::TokioAsyncResolver::tokio_from_system_conf() {
        Ok(x) => Ok(x),
        Err(_) => Err("Could not build dns resolver".to_string()),
    }
}

/// Funtion to start crawling when depth is specified.
/// Does not use Sitemaps.
pub async fn crawl_with_depth(
    origin_url: Link,
    crawl_depth: usize,
    whitelist: Option<HashSet<url::Host>>,
    blacklist: Option<HashSet<url::Host>>,
    word_list: HashSet<String>,
    tx_output: mpsc::Sender<Link>,
    tx_selenium: mpsc::Sender<String>,
    task_limit: usize,
    timeout: u64,
) -> Result<(), String> {
    let mut to_crawl: HashSet<Url> = HashSet::new();
    let mut crawled: HashSet<Url> = HashSet::new();
    let mut dont_crawl: HashSet<Url> = HashSet::new();
    let word_list = Arc::new(word_list);

    let client = init_reqwest_client(timeout)?;
    let resolver = init_dns_resolver()?;

    to_crawl.insert(origin_url.url);

    for _ in 0..crawl_depth {
        println!("Crawling {} URls", to_crawl.len());

        let (tx_cralwer, mut rx_crawler) = mpsc::channel::<Link>(task_limit);

        to_crawl.iter().cloned().for_each(|x| {
            let tx_clone = tx_cralwer.clone();
            let tx_selenium_clone = tx_selenium.clone();
            let client_clone = client.clone();
            let resolver_clone = resolver.clone();
            let word_list_clone = word_list.clone();
            tokio::spawn(async move {
                crawl_page(
                    x,
                    client_clone,
                    tx_clone,
                    tx_selenium_clone,
                    task_limit,
                    resolver_clone,
                    word_list_clone,
                )
                .await
            });
        });

        to_crawl.clear();

        drop(tx_cralwer);

        while let Some(link) = rx_crawler.recv().await {
            if link.crawled {
                crawled.insert(link.url.clone());
                if let Err(_) = tx_output.send(link).await {
                    return Err("Output Connection Closed".to_string());
                }
            } else {
                let should_crawl = link.should_crawl(&whitelist, &blacklist);
                if should_crawl && !crawled.contains(&link.url) {
                    to_crawl.insert(link.url);
                } else if !should_crawl && !dont_crawl.contains(&link.url) {
                    dont_crawl.insert(link.url.clone());
                    if let Err(_) = tx_output.send(link).await {
                        return Err("Output Connection Closed".to_string());
                    }
                }
            }
        }
    }

    stream::iter(to_crawl)
        .map(|x| links::Link::new_from_url(&x))
        .for_each_concurrent(task_limit, |x| async {
            let _ = tx_output.send(x).await;
        })
        .await;
    Ok(())
}

/// Function to crawl when depth is not specified.
/// Also makes use of Sitemaps.
pub async fn crawl_no_depth(
    origin_url: Link,
    whitelist: Option<HashSet<url::Host>>,
    blacklist: Option<HashSet<url::Host>>,
    word_list: HashSet<String>,
    tx_output: mpsc::Sender<Link>,
    tx_selenium: mpsc::Sender<String>,
    task_limit: usize,
    timeout: u64,
) -> Result<(), String> {
    let mut to_crawl: HashSet<Url> = HashSet::new();
    let mut crawled: HashSet<Url> = HashSet::new();
    let mut dont_crawl: HashSet<Url> = HashSet::new();
    let word_list = Arc::new(word_list);

    let client = init_reqwest_client(timeout)?;
    let resolver = init_dns_resolver()?;

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

        to_crawl.iter().cloned().for_each(|x| {
            let tx_clone = tx_cralwer.clone();
            let tx_selenium_clone = tx_selenium.clone();
            let client_clone = client.clone();
            let resolver_clone = resolver.clone();
            let word_list_clone = word_list.clone();
            tokio::spawn(async move {
                crawl_page(
                    x,
                    client_clone,
                    tx_clone,
                    tx_selenium_clone,
                    task_limit,
                    resolver_clone,
                    word_list_clone,
                )
                .await
            });
        });

        to_crawl.clear();

        drop(tx_cralwer);

        while let Some(link) = rx_crawler.recv().await {
            if link.crawled {
                crawled.insert(link.url.clone());
                if let Err(_) = tx_output.send(link).await {
                    return Err("Output Connection Closed".to_string());
                }
            } else {
                let should_crawl = link.should_crawl(&whitelist, &blacklist);
                if should_crawl && !crawled.contains(&link.url) {
                    to_crawl.insert(link.url);
                } else if !should_crawl && !dont_crawl.contains(&link.url) {
                    dont_crawl.insert(link.url.clone());
                    if let Err(_) = tx_output.send(link).await {
                        return Err("Output Connection Closed".to_string());
                    }
                }
            }
        }
    }

    Ok(())
}

/// Function to handle crawling a single page.
/// Is Single Threaded.
async fn crawl_page(
    url: Url,
    client: reqwest::Client,
    tx: mpsc::Sender<Link>,
    tx_selenium: mpsc::Sender<String>,
    limit: usize,
    resolver: trust_dns_resolver::TokioAsyncResolver,
    word_list: Arc<HashSet<String>>,
) {
    let mut link = links::Link::new_from_url(&url);
    let resp = match get_page(url.as_str(), &client).await {
        Ok(x) => x,
        Err(_) => {
            link.crawled = true;
            let _ = tx.send(link.clone()).await;

            return;
        }
    };
    link.update_from_response(&resp);
    if let Some(host) = &link.host {
        let host = host.to_string();
        let ipv4 = links::resolve_ipv4(&resolver, &host).await;
        let ipv6 = links::resolve_ipv6(&resolver, &host).await;
        link.update_dns(ipv4, ipv6);
    };
    let is_html = link.check_mime_from_list(&[mime::TEXT_HTML, mime::TEXT_HTML_UTF_8]);

    if is_html {
        let html = match resp.text().await {
            Ok(x) => x,
            Err(_) => {
                return;
            }
        };
        if links::check_words_html(&html, word_list) {
            link.contains_words = true;
            let _ = tx_selenium.send(link.url.to_string()).await;
        }

        let links = links::get_links_from_html(&html, url.as_str());
        let tx_ref = &tx;
        stream::iter(links)
            .for_each_concurrent(limit, |x| async move {
                let _ = tx_ref.send(x).await;
            })
            .await;
    }

    if let Err(_) = tx.send(link).await {
        return;
    }
}

/// Function to find and crawl sitemaps from robottxt.
async fn crawl_sitemaps(url: Url, tx: mpsc::Sender<Link>, limit: usize, client: reqwest::Client) {
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

/// Function to crawl a single sitemap.
/// Currently only supports text sitemap
async fn crawl_sitemap(url: Url, tx: mpsc::Sender<Link>, limit: usize, client: reqwest::Client) {
    let mut link = links::Link::new_from_url(&url);
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

/// Funtion to perform a get request on a url
async fn get_page(
    url: &str,
    client: &reqwest::Client,
) -> Result<reqwest::Response, reqwest::Error> {
    let resp = client.get(url).send().await?;
    resp.error_for_status()
}
