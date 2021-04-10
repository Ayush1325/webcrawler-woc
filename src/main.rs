/*!
# Introduction
- The project basically crawls the webpage and collects as much information as possible,
like external links, mails, etc. Like a web crawler used by search engines but specific for
a domain and url.
- It is a project for WOC.
# CLI Usage
```
webcrawler 1.0
Ayush Singh <ayushsingh1325@gmail.com>

USAGE:
    webcrawler [FLAGS] [OPTIONS] <url>

ARGS:
    <url>    Seed url for crawler

FLAGS:
    -h, --help        Prints help information
        --selenium    Flag for taking screenshots using Selenium. Takes screenshot if a word from
                      wordlist is found in the page
        --verbose     Output the link to standard output
    -V, --version     Prints version information

OPTIONS:
    -b, --blacklist <blacklist>            Path of file containing list of domains not to be crawled
    -d, --depth <depth>                    Gives numeric depth for crawl
    -o, --output-folder <output-folder>    Path to the output folder
    -s, --search-words <search-words>      Path to file containing words to search for in the page
        --task-limit <task-limit>          Limits the number of parallel tasks [default: 1000]
    -t, --timeout <timeout>                Timout for http requests [default: 10]
    -w, --whitelist <whitelist>            Path of file containing list of domains to be crawled
```
*/
mod cli;
mod crawler;
mod extractors;
mod file_handler;

#[tokio::main]
async fn main() {
    cli::entry().await;
    // test().await;
}

#[allow(dead_code)]
#[cfg(debug_assertions)]
async fn test() {
    use url::Url;

    let t = Url::parse("tel:+6494461709").unwrap();
    println!("{}", t.scheme() == "tel");
}
