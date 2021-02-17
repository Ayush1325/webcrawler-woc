use crate::cli::CrawlDepth;
use crate::extractors::links;
use futures::future;
use reqwest::Url;
use std::collections::HashSet;

pub async fn crawl_host(
    origin_url: String,
    crawl_depth: CrawlDepth,
) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    //! Crawls all links in the same host.
    //! TODO: Fix copying links all over the place to reduce the memory usage.
    let mut visited = HashSet::new();
    let mut found_links = HashSet::new();
    let mut to_crawl = HashSet::new();

    to_crawl.insert(origin_url.to_string());

    #[cfg(debug_assertions)]
    let mut count: usize = 1;

    while !to_crawl.is_empty() {
        #[cfg(debug_assertions)]
        {
            println!("Pass: {}, Tasks: {}", count, to_crawl.len());
            count += 1;
        }
        let handles = to_crawl
            .iter()
            .map(|x| tokio::spawn(crawl_page(x.to_string())));
        let response = future::join_all(handles).await;
        let new_links: HashSet<String> = response.iter().fold(HashSet::new(), |mut acc, x| {
            if let Ok(Ok(links)) = x {
                acc.extend(links.clone());
            }
            acc
        });
        found_links.extend(new_links.clone());
        visited.extend(to_crawl.clone());
        to_crawl = new_links
            .difference(&visited)
            .filter(|x| match crawl_depth {
                CrawlDepth::Page => false,
                CrawlDepth::Variable(depth) => compare_depth(x, depth),
                CrawlDepth::Domain => compare_host(origin_url.as_str(), x),
            })
            .map(|x| x.to_string())
            .collect();
    }
    Ok(found_links)
}

async fn crawl_page(url: String) -> Result<HashSet<String>, reqwest::Error> {
    //! Function to crawl a single page.
    //! Combines get_page() and get_links_from_html().
    //! Uses blocking threadpool for extracting links.
    //! TODO: Remove Coupling from crawl_host() so that it can be used independently.
    let page = get_page(&url).await?;
    let html = page.text().await?;
    let links = tokio::task::spawn_blocking(move || {
        links::get_links_from_html(html.as_str(), url.as_str())
    })
    .await;
    match links {
        Ok(links) => Ok(links),
        Err(_) => Ok(HashSet::new()),
    }
}

pub async fn get_page(url: &str) -> Result<reqwest::Response, reqwest::Error> {
    //! Function to make get request to a single url.
    let resp = reqwest::get(url).await?;
    resp.error_for_status()
}

fn compare_host(original_url: &str, url: &str) -> bool {
    //! Helper function to compare hosts.
    let original_url = Url::parse(original_url);
    let url = Url::parse(url);
    if let (Ok(url1), Ok(url2)) = (original_url, url) {
        if let (Some(url1_host), Some(url2_host)) = (url1.host_str(), url2.host_str()) {
            if url1_host == url2_host {
                return true;
            }
        }
    }
    false
}

fn compare_depth(url: &str, depth: usize) -> bool {
    let url = Url::parse(url);
    if let Ok(parsed_url) = url {
        if let Some(d) = parsed_url.path_segments() {
            if d.count() <= depth {
                return true;
            }
        }
    }
    false
}
