use reqwest::StatusCode;
use reqwest::Url;
use select::document::Document;
use select::predicate::Name;
use std::collections::HashSet;

fn get_page(url: &Url) -> Result<reqwest::blocking::Response, Box<dyn std::error::Error>> {
    //! Function to make get request to a single url.
    //! Impure Function
    let resp = reqwest::blocking::get(url.clone())?;
    let status = resp.status();
    if status != StatusCode::OK {
        let err = Box::from(format!("GET Error Code: {}", status.as_u16()));
        return Err(err);
    }
    Ok(resp)
}

fn crawl_url(url: &Url) {
    // TODO
}

fn get_links_from_html(html: &str, url: &Url) -> HashSet<Url> {
    //! Function to extract all links from a given html string.
    //! Pure function
    Document::from(html)
        .find(Name("a"))
        .filter_map(|n| n.attr("href"))
        .filter_map(|x| normalize_url(x, url))
        .collect::<HashSet<Url>>()
}

fn normalize_url(url: &str, base_url: &Url) -> Option<Url> {
    //! Helper function to parse url in a page.
    //! Converts relative urls to full urls.
    //! Also removes javascript urls and other false urls.
    //! Pure Function
    if url.starts_with("#") {
        // Checks for internal links.
        // Maybe will make it optioanl to ignore them.
        return None;
    }

    let new_url = Url::parse(url);
    match new_url {
        Ok(new_url) => Some(new_url),
        Err(_) => {
            let new_url = base_url.join(url);
            match new_url {
                Ok(x) => Some(x),
                Err(_) => None,
            }
        }
    }
}

pub fn temp() -> () {
    let url = Url::parse("https://www.wikipedia.org/").unwrap();
    let page = get_page(&url).unwrap();
    let html = page.text().unwrap();
    let links = get_links_from_html(&html, &url);
    links.iter().for_each(|x| println!("{}", x.as_str()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_hasset(arr: Vec<&str>) -> HashSet<Url> {
        arr.iter().map(|x| Url::parse(x).unwrap()).collect()
    }

    #[test]
    fn simple_html() {
        let html = "<a href='/123.html'></a>
                    <a href='#1'></a>
                    <a href='123.html'></a>
                    <a href='https://test2.com'></a>";
        let url = Url::parse("https://test.com/home/").unwrap();
        let links = get_links_from_html(html, &url);
        let ans = gen_hasset(vec![
            "https://test.com/123.html",
            "https://test.com/home/123.html",
            "https://test2.com",
        ]);
        assert_eq!(links, ans);
    }
}
