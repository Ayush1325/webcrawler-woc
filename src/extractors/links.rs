use mime::Mime;
use reqwest::Url;
use select::document::Document;
use select::predicate::Name;
use std::hash::Hasher;
use std::{collections::HashSet, hash::Hash};

#[derive(Clone, Debug)]
pub struct Link {
    pub url: String,
    pub host: Option<String>,
    depth: Option<usize>,
    found_at: HashSet<String>,
    content_type: Option<Mime>,
    headers: Option<reqwest::header::HeaderMap>,
    pub crawled: bool,
}

impl Link {
    pub fn new(url: &str, found_at: Option<&str>) -> Option<Self> {
        let parsed_url = match Url::parse(url) {
            Ok(x) => x,
            Err(_) => return None,
        };
        let host = match parsed_url.host_str() {
            Some(x) => Some(x.to_string()),
            None => None,
        };
        let depth = match parsed_url.path_segments() {
            Some(x) => Some(x.count()),
            None => None,
        };
        let mut temp = HashSet::new();
        if let Some(x) = found_at {
            temp.insert(x.to_string());
        }
        Some(Link {
            url: url.to_string(),
            host,
            depth,
            found_at: temp,
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
            Ok(x) => Self::new(x.as_str(), Some(base_url)),
            Err(_) => None,
        }
    }

    pub fn from_response(response: &reqwest::Response) -> Option<Self> {
        let host = match response.url().host_str() {
            Some(x) => Some(x.to_string()),
            None => None,
        };
        let depth = match response.url().path_segments() {
            Some(x) => Some(x.count()),
            None => None,
        };
        Some(Link {
            url: response.url().to_string(),
            host,
            depth,
            found_at: HashSet::new(),
            content_type: Self::get_mime(response.headers()),
            headers: Some(response.headers().to_owned()),
            crawled: true,
        })
    }

    pub fn should_crawl(&self, depth: Option<usize>, required_host: &str) -> bool {
        match depth {
            Some(x) => match self.depth {
                Some(y) => y <= x,
                None => false,
            },
            None => Self::check_host(self, required_host),
        }
    }

    fn check_host(&self, required_host: &str) -> bool {
        match &self.host {
            Some(x) => x.as_str() == required_host,
            None => false,
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

pub fn get_links_from_html(html: &str, url: String) -> HashSet<Link> {
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

    match Link::new(url, None) {
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
