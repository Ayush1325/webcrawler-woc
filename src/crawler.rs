use crate::extractors::links;
use futures::future;
use std::collections::HashSet;

pub async fn crawl(
    origin_url: String,
    crawl_depth: Option<usize>,
) -> Result<HashSet<links::Link>, String> {
    let mut to_crawl: HashSet<links::Link> = HashSet::new();
    let mut crawled: HashSet<links::Link> = HashSet::new();
    let mut dont_crawl: HashSet<links::Link> = HashSet::new();

    let origin_url = match links::Link::new(&origin_url, None) {
        Some(x) => x,
        None => return Err("Invalid URL".to_string()),
    };
    to_crawl.insert(origin_url.clone());

    let origin_host = match origin_url.host {
        Some(x) => x,
        None => return Err("Invalid URL Host".to_string()),
    };

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
        crawled.extend(temp_links.iter().map(|x| x.0.clone()));
        let temp_found_links =
            temp_links
                .iter()
                .map(|x| x.1.clone())
                .fold(HashSet::new(), |mut acc, x| {
                    acc.extend(x);
                    acc
                });
        to_crawl = temp_found_links
            .iter()
            .filter(|x| x.should_crawl(crawl_depth, &origin_host))
            .cloned()
            .collect::<HashSet<links::Link>>();
        dont_crawl = temp_found_links
            .iter()
            .filter(|x| !x.should_crawl(crawl_depth, &origin_host))
            .cloned()
            .collect();
    }

    Ok(crawled.union(&dont_crawl).cloned().collect())
}

pub async fn crawl_page(link: &links::Link) -> (links::Link, HashSet<links::Link>) {
    let mut link = link.clone();
    let response = match get_page(&link.url).await {
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

pub async fn get_page(url: &str) -> Result<reqwest::Response, reqwest::Error> {
    //! Function to make get request to a single url.
    let resp = reqwest::get(url).await?;
    resp.error_for_status()
}
