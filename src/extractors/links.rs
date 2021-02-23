use mime::Mime;
use reqwest::Url;
use select::document::Document;
use select::predicate::Name;
use std::hash::Hasher;
use std::{collections::HashSet, hash::Hash, sync::Arc};

#[derive(Clone, Debug)]
pub struct Link {
    pub url: Arc<String>,
    pub host: Option<String>,
    depth: Option<usize>,
    content_type: Option<Mime>,
    headers: Option<reqwest::header::HeaderMap>,
    pub crawled: bool,
}

impl Link {
    pub fn new(url: &str) -> Option<Self> {
        let parsed_url = match Url::parse(url) {
            Ok(x) => x,
            Err(_) => return None,
        };
        let host = match parsed_url.host_str() {
            Some(x) => Some(x.to_string()),
            None => None,
        };
        Some(Link {
            url: Arc::new(url.to_string()),
            host,
            depth: Self::get_depth(&parsed_url),
            content_type: None,
            headers: None,
            crawled: false,
        })
    }

    pub fn new_relative(url: &str, base_url: &str) -> Option<Self> {
        let base_url_parsed = match Url::parse(base_url) {
            Ok(x) => x,
            Err(_) => return None,
        };
        match base_url_parsed.join(url) {
            Ok(x) => Self::new(x.as_str()),
            Err(_) => None,
        }
    }

    pub fn from_response(response: &reqwest::Response) -> Option<Self> {
        let host = match response.url().host_str() {
            Some(x) => Some(x.to_string()),
            None => None,
        };
        Some(Link {
            url: Arc::new(response.url().to_string()),
            host,
            depth: Self::get_depth(response.url()),
            content_type: Self::get_mime(response.headers()),
            headers: Some(response.headers().to_owned()),
            crawled: true,
        })
    }

    fn get_depth(url: &Url) -> Option<usize> {
        match url.path_segments() {
            Some(x) => Some(x.count()),
            None => None,
        }
    }

    pub fn should_crawl(
        &self,
        depth: Option<usize>,
        whitelist_host: &Option<HashSet<String>>,
        blacklist_host: &Option<HashSet<String>>,
    ) -> bool {
        if let Some(x) = whitelist_host {
            return self.check_host(x, false);
        }
        if let Some(x) = blacklist_host {
            return !self.check_host(x, true);
        }
        match depth {
            Some(x) => match self.depth {
                Some(y) => y <= x,
                None => false,
            },
            None => false,
        }
    }

    fn check_host(&self, required_host: &HashSet<String>, default: bool) -> bool {
        match &self.host {
            Some(x) => required_host.contains(x),
            None => default,
        }
    }

    pub fn update_from_response(&mut self, response: &reqwest::Response) {
        self.content_type = Self::get_mime(response.headers());
        self.headers = Some(response.headers().to_owned());
        self.crawled = true;
    }

    fn get_mime(header: &reqwest::header::HeaderMap) -> Option<Mime> {
        let mime_str = header.get(reqwest::header::CONTENT_TYPE)?.to_str();
        if let Ok(mime_type) = mime_str {
            let mime_type = mime_type.parse::<Mime>();
            if let Ok(t) = mime_type {
                return Some(t);
            }
        }
        None
    }

    pub fn check_html(&self) -> bool {
        if let Some(c) = &self.content_type {
            let content_type = c.type_();
            let content_subtype = c.subtype();
            if content_type == mime::HTML
                || (content_type == mime::TEXT && content_subtype == mime::HTML)
            {
                return true;
            }
        }
        false
    }
}

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl Eq for Link {}

impl Hash for Link {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

pub fn get_links_from_html(html: &str, url: Arc<String>) -> HashSet<Link> {
    //! Function to extract all links from a given html string.
    let url_parsed = Url::parse(&url);
    match url_parsed {
        Ok(url) => Document::from(html)
            .find(Name("a"))
            .filter_map(|n| n.attr("href"))
            .filter_map(|x| normalize_url(x, url.as_str()))
            .collect::<HashSet<Link>>(),
        Err(_) => HashSet::new(),
    }
}

fn normalize_url(url: &str, base_url: &str) -> Option<Link> {
    //! Helper function to parse url in a page.
    //! Converts relative urls to full urls.
    //! Also removes javascript urls and other false urls.
    if url.starts_with("#") {
        // Checks for internal links.
        // Maybe will make it optioanl to ignore them.
        return None;
    }

    match Link::new(url) {
        Some(x) => Some(x),
        None => Link::new_relative(url, base_url),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_hasset(arr: Vec<&str>) -> HashSet<Arc<String>> {
        arr.iter().map(|x| Arc::new(x.to_string())).collect()
    }

    #[test]
    fn simple_html() {
        let html = "<a href='/123.html'></a>
                    <a href='#1'></a>
                    <a href='123.html'></a>
                    <a href='https://test2.com'></a>";
        let url = Arc::new("https://test.com/home/".to_string());
        let links = get_links_from_html(html, url);
        let ans = gen_hasset(vec![
            "https://test.com/123.html",
            "https://test.com/home/123.html",
            "https://test2.com/",
        ]);
        assert_eq!(links, ans);
    }
}
