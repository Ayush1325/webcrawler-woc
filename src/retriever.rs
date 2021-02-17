use futures::future;
use reqwest::Url;
use select::document::Document;
use select::predicate::Name;
use std::collections::HashSet;

enum CrawlDepth {
    Zero,
    Variable(usize),
    Domain,
}

async fn get_page(url: &str) -> Result<reqwest::Response, reqwest::Error> {
    //! Function to make get request to a single url.
    let resp = reqwest::get(url).await?;
    resp.error_for_status()
}

async fn crawl_page(url: String) -> Result<HashSet<String>, reqwest::Error> {
    //! Function to crawl a single page.
    //! Combines get_page() and get_links_from_html().
    //! Uses blocking threadpool for extracting links.
    //! TODO: Remove Coupling from crawl_host() so that it can be used independently.
    let page = get_page(&url).await?;
    let html = page.text().await?;
    let links =
        tokio::task::spawn_blocking(move || get_links_from_html(html.as_str(), url.as_str())).await;
    match links {
        Ok(links) => Ok(links),
        Err(_) => Ok(HashSet::new()),
    }
}

async fn crawl_host(
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
                CrawlDepth::Zero => false,
                CrawlDepth::Variable(depth) => compare_depth(x, depth),
                CrawlDepth::Domain => compare_host(origin_url.as_str(), x),
            })
            .map(|x| x.to_string())
            .collect();
    }
    Ok(found_links)
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

fn get_links_from_html(html: &str, url: &str) -> HashSet<String> {
    //! Function to extract all links from a given html string.
    let url_parsed = Url::parse(url);
    match url_parsed {
        Ok(url) => Document::from(html)
            .find(Name("a"))
            .filter_map(|n| n.attr("href"))
            .filter_map(|x| normalize_url(x, &url))
            .collect::<HashSet<String>>(),
        Err(_) => HashSet::new(),
    }
}

fn normalize_url(url: &str, base_url: &Url) -> Option<String> {
    //! Helper function to parse url in a page.
    //! Converts relative urls to full urls.
    //! Also removes javascript urls and other false urls.
    if url.starts_with("#") {
        // Checks for internal links.
        // Maybe will make it optioanl to ignore them.
        return None;
    }

    let new_url = Url::parse(url);
    match new_url {
        Ok(new_url) => Some(new_url.to_string()),
        Err(_) => {
            let new_url = base_url.join(url);
            match new_url {
                Ok(x) => Some(x.to_string()),
                Err(_) => None,
            }
        }
    }
}

pub async fn temp() -> () {
    // let url = "https://www.wikipedia.org/";
    // let page = get_page(&url).await.unwrap();
    // let html = page.text().await.unwrap();
    // let links = get_links_from_html(&html, &url);
    // links.iter().for_each(|x| println!("{}", x.as_str()));
    let origin_url = "https://crawler-test.com/".to_string();
    // let local_url = "http://127.0.0.1:5500/index.html".to_string();
    let links = crawl_host(origin_url, CrawlDepth::Variable(1))
        .await
        .unwrap();
    links.iter().for_each(|x| println!("{}", x));
    // let url = Url::parse("https://www.wikipedia.org/home.html").unwrap();
    // println!("{}", url.path_segments().unwrap().count());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_hasset(arr: Vec<&str>) -> HashSet<String> {
        arr.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn simple_html() {
        let html = "<a href='/123.html'></a>
                    <a href='#1'></a>
                    <a href='123.html'></a>
                    <a href='https://test2.com'></a>";
        let url = "https://test.com/home/";
        let links = get_links_from_html(html, &url);
        let ans = gen_hasset(vec![
            "https://test.com/123.html",
            "https://test.com/home/123.html",
            "https://test2.com/",
        ]);
        assert_eq!(links, ans);
    }
}
