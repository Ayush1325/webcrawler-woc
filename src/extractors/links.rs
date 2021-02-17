use reqwest::Url;
use select::document::Document;
use select::predicate::Name;
use std::collections::HashSet;

pub fn get_links_from_html(html: &str, url: &str) -> HashSet<String> {
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
