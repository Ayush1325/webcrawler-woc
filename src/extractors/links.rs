use mime::Mime;
use reqwest::Url;
use select::{document::Document, predicate::Name};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet, fmt, hash::Hash, hash::Hasher, net::Ipv4Addr, net::Ipv6Addr, sync::Arc,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LinkType {
    Mail,
    PhoneNo,
    Other,
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    ipv4: Option<Ipv4Addr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ipv6: Option<Ipv6Addr>,
    pub link_type: LinkType,
    pub contains_words: bool,
}

impl Link {
    pub fn new(
        url: &Url,
        headers: &Option<reqwest::header::HeaderMap>,
        ipv4: Option<Ipv4Addr>,
        ipv6: Option<Ipv6Addr>,
        crawled: bool,
        link_type: LinkType,
        contains_words: bool,
    ) -> Self {
        let host = match url.host() {
            Some(x) => Some(x.to_owned()),
            None => None,
        };
        let content_type = match headers {
            Some(x) => Self::get_mime(x),
            None => None,
        };
        Link {
            url: url.to_owned(),
            headers: headers.to_owned(),
            content_type,
            host,
            ipv4,
            ipv6,
            crawled,
            link_type,
            contains_words,
        }
    }

    pub fn new_from_str(url: &str) -> Option<Self> {
        let parsed_url = match Url::parse(url) {
            Ok(x) => x,
            Err(_) => return None,
        };
        Some(Self::new(
            &parsed_url,
            &None,
            None,
            None,
            false,
            Self::get_link_type(&parsed_url),
            false,
        ))
    }

    pub fn new_from_url(url: &Url) -> Self {
        Self::new(
            url,
            &None,
            None,
            None,
            false,
            Self::get_link_type(url),
            false,
        )
    }

    pub fn new_relative(url: &str, base_url: &str) -> Option<Self> {
        let base_url_parsed = match Url::parse(base_url) {
            Ok(x) => x,
            Err(_) => return None,
        };
        match base_url_parsed.join(url) {
            Ok(x) => Self::new_from_str(x.as_str()),
            Err(_) => None,
        }
    }

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

    fn get_link_type(url: &Url) -> LinkType {
        match url.scheme() {
            "mailto" => LinkType::Mail,
            "tel" => LinkType::PhoneNo,
            _ => LinkType::Other,
        }
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

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = match serde_json::to_string_pretty(self) {
            Ok(x) => x,
            Err(_) => return Err(fmt::Error),
        };
        write!(f, "{}", json)
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

pub fn check_words_html(html: &str, word_list: Arc<HashSet<String>>) -> bool {
    word_list
        .iter()
        .find(|x| html.contains(x.as_str()))
        .is_some()
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

    match Link::new_from_str(&url) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, sync::Arc};

    #[test]
    fn get_words() {
        let html = "This is a sample page which does not work";
        let mut word_list = HashSet::new();

        assert!(!check_words_html(html, Arc::new(word_list.clone())));

        word_list.insert("sample".to_string());
        assert!(check_words_html(html, Arc::new(word_list)))
    }
}
