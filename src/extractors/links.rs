use mime::Mime;
use reqwest::Url;
use select::{document::Document, predicate::Name};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, hash::Hash, hash::Hasher, net::Ipv4Addr, net::Ipv6Addr};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Link {
    pub url: Url,
    #[serde(skip)]
    pub host: Option<url::Host>,
    #[serde(with = "opt_mime", skip_serializing_if = "Option::is_none")]
    pub content_type: Option<Mime>,
    #[serde(with = "opt_headermap", skip_serializing_if = "Option::is_none")]
    headers: Option<reqwest::header::HeaderMap>,
    #[serde(skip)]
    pub crawled: bool,
    pub ipv4: Option<Ipv4Addr>,
    pub ipv6: Option<Ipv6Addr>,
}

impl Link {
    pub fn new(url: &str) -> Option<Self> {
        let parsed_url = match Url::parse(url) {
            Ok(x) => x,
            Err(_) => return None,
        };
        let host = match parsed_url.host() {
            Some(x) => Some(x.to_owned()),
            None => None,
        };
        Some(Link {
            url: parsed_url,
            host,
            content_type: None,
            headers: None,
            crawled: false,
            ipv4: None,
            ipv6: None,
        })
    }

    pub fn from_url(url: &Url) -> Self {
        Link {
            url: url.clone(),
            host: match url.host() {
                Some(x) => Some(x.to_owned()),
                None => None,
            },
            content_type: None,
            headers: None,
            crawled: false,
            ipv4: None,
            ipv6: None,
        }
    }

    // pub fn from_response(url: &reqwest::Response) -> Self {
    //     Link {
    //         url: url.url().to_owned(),
    //         host: match url.url().host() {
    //             Some(x) => Some(x.to_owned()),
    //             None => None,
    //         },
    //         content_type: Self::get_mime(url.headers()),
    //         headers: Some(url.headers().to_owned()),
    //         crawled: true,
    //     }
    // }

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

    // fn get_depth(url: &Url) -> Option<usize> {
    //     match url.path_segments() {
    //         Some(x) => Some(x.count()),
    //         None => None,
    //     }
    // }

    pub fn should_crawl(
        &self,
        whitelist_host: &Option<HashSet<url::Host>>,
        blacklist_host: &Option<HashSet<url::Host>>,
    ) -> bool {
        if let Some(x) = whitelist_host {
            return self.check_host(x, false);
        }
        if let Some(x) = blacklist_host {
            return !self.check_host(x, true);
        }
        false
    }

    fn check_host(&self, required_host: &HashSet<url::Host>, default: bool) -> bool {
        match &self.host {
            Some(x) => required_host.contains(x),
            None => default,
        }
    }

    pub fn update_dns(&mut self, ipv4: Option<Ipv4Addr>, ipv6: Option<Ipv6Addr>) {
        self.ipv4 = ipv4;
        self.ipv6 = ipv6;
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

    pub fn check_mime_from_list(&self, required_mime: &[Mime]) -> bool {
        if let Some(c) = &self.content_type {
            return required_mime.iter().any(|x| x == c);
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

mod opt_mime {
    use mime::Mime;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<Mime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(x) => hyper_serde::serialize(x, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Mime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match hyper_serde::deserialize(deserializer) {
            Ok(x) => Ok(Some(x)),
            Err(_) => Ok(None),
        }
    }
}

mod opt_headermap {
    use reqwest::header::HeaderMap;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<HeaderMap>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(x) => http_serde::header_map::serialize(x, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<HeaderMap>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match http_serde::header_map::deserialize(deserializer) {
            Ok(x) => Ok(Some(x)),
            Err(_) => Ok(None),
        }
    }
}

pub fn get_links_from_html(html: &str, url: &str) -> HashSet<Link> {
    Document::from(html)
        .find(Name("a"))
        .filter_map(|x| x.attr("href"))
        .filter_map(|x| normalize_url(x, url))
        .collect()
}

pub fn get_links_from_text(text: &str, url: &str) -> HashSet<Link> {
    text.lines()
        .map(|x| x.trim())
        .filter_map(|x| normalize_url(x, url))
        .collect()
}

pub fn normalize_url(url: &str, base_url: &str) -> Option<Link> {
    //! Helper function to parse url in a page.
    //! Converts relative urls to full urls.
    //! Also removes javascript urls and other false urls.
    if url.starts_with("#") {
        // Checks for internal links.
        // Maybe will make it optioanl to ignore them.
        return None;
    }

    match Link::new(&url) {
        Some(x) => Some(x),
        None => Link::new_relative(&url, base_url),
    }
}

pub async fn resolve_ipv4(
    resolver: &trust_dns_resolver::TokioAsyncResolver,
    query: &str,
) -> Option<Ipv4Addr> {
    match resolver.ipv4_lookup(query).await {
        Ok(x) => match x.iter().next() {
            Some(x) => Some(x.to_owned()),
            None => None,
        },
        Err(_) => None,
    }
}

pub async fn resolve_ipv6(
    resolver: &trust_dns_resolver::TokioAsyncResolver,
    query: &str,
) -> Option<Ipv6Addr> {
    match resolver.ipv6_lookup(query).await {
        Ok(x) => match x.iter().next() {
            Some(x) => Some(x.to_owned()),
            None => None,
        },
        Err(_) => None,
    }
}
