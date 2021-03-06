/*!
Module Containing the Command Line part of the Crawler.
*/
use crate::extractors::links::Link;
use crate::file_handler;
use clap::Clap;
use std::{collections::HashSet, path::PathBuf, time::Instant};
use tokio::sync::mpsc;

#[derive(Clap, Clone)]
#[clap(version = "1.0", author = "Ayush Singh <ayushsingh1325@gmail.com>")]
struct CLI {
    /// Seed url for crawler.
    url: String,
    /// Gives numeric depth for crawl.
    #[clap(short, long)]
    depth: Option<usize>,
    /// Path of file containing list of domains to be crawled.
    #[clap(short, long)]
    whitelist: Option<PathBuf>,
    /// Path of file containing list of domains not to be crawled.
    #[clap(short, long)]
    blacklist: Option<PathBuf>,
    /// Path to file containing words to search for in the page.
    #[clap(short, long)]
    search_words: Option<PathBuf>,
    /// Path to the output folder.
    #[clap(short, long)]
    output_folder: Option<PathBuf>,
    /// Output the link to standard output.
    #[clap(long)]
    verbose: bool,
    /// Timout for http requests.
    #[clap(short, long, default_value = "10")]
    timeout: u64,
    /// Flag for taking screenshots using Selenium.
    /// Takes screenshot if a word from wordlist is found in the page.
    #[clap(long)]
    selenium: bool,
    /// Limits the number of parallel tasks.
    #[clap(long, default_value = "1000")]
    task_limit: usize,
}

/// Funtion that servers as the entry point to the Command Line Tool.
/// It parses the arguments and fires off the methods.
pub async fn entry() {
    let start_time = Instant::now();
    let opts = CLI::parse();
    println!("Started");
    let task_limit = opts.task_limit;
    let (tx_output, rx_output) = mpsc::channel(task_limit);
    let (tx_selenium, rx_selenium) = mpsc::channel(task_limit);

    let output_folder = opts.output_folder.clone();
    let verbose = opts.verbose;
    let url = opts.url.clone();
    let depth = opts.depth;
    let whitelist = opts.whitelist;
    let blacklist = opts.blacklist;
    let search_words = opts.search_words;
    let timeout = opts.timeout;
    let selenium = opts.selenium;

    let output_folder_clone = output_folder.clone();

    let output_handler = tokio::spawn(async move {
        handle_output(output_folder_clone, verbose, rx_output, task_limit).await
    });

    let crawler_handler = tokio::spawn(async move {
        launch_crawler(
            url,
            depth,
            task_limit,
            tx_output,
            tx_selenium,
            whitelist,
            blacklist,
            search_words,
            timeout,
        )
        .await
    });

    let output_folder_clone = output_folder.clone();
    let selenium_handler =
        tokio::spawn(
            async move { handle_selenium(output_folder_clone, selenium, rx_selenium).await },
        );

    let returns =
        futures::future::try_join3(output_handler, crawler_handler, selenium_handler).await;

    if let Err(x) = returns {
        println!("Error : {}", x);
    }

    println!("Time Taken: {} seconds", start_time.elapsed().as_secs());
}

/// Funtion to launch the crawler. Fires off the correct crawler method depending on the arguments.
async fn launch_crawler(
    origin_url: String,
    depth: Option<usize>,
    task_limit: usize,
    tx_output: mpsc::Sender<Link>,
    tx_selenium: mpsc::Sender<String>,
    whitelist: Option<PathBuf>,
    blacklist: Option<PathBuf>,
    search_words: Option<PathBuf>,
    timeout: u64,
) -> Result<(), String> {
    let origin_url = match Link::new_from_str(origin_url.as_str()) {
        Some(x) => x,
        None => return Err("Invalid Url".to_string()),
    };

    let whitelist = match whitelist {
        Some(x) => match file_handler::read_hosts(x).await {
            Ok(y) => Some(y),
            Err(_) => return Err("Error in reading Whitelist".to_string()),
        },
        None => None,
    };

    let blacklist = match blacklist {
        Some(x) => match file_handler::read_hosts(x).await {
            Ok(y) => Some(y),
            Err(_) => return Err("Error in reading Blacklist".to_string()),
        },
        None => None,
    };

    let word_list = match search_words {
        Some(x) => match file_handler::read_words(x).await {
            Ok(x) => x,
            Err(_) => return Err("Error in reading Word List".to_string()),
        },
        None => HashSet::new(),
    };

    let handler = match depth {
        None => {
            crate::crawler::crawl_no_depth(
                origin_url,
                whitelist,
                blacklist,
                word_list,
                tx_output,
                tx_selenium,
                task_limit,
                timeout,
            )
            .await
        }

        Some(x) => {
            crate::crawler::crawl_with_depth(
                origin_url,
                x,
                whitelist,
                blacklist,
                word_list,
                tx_output,
                tx_selenium,
                task_limit,
                timeout,
            )
            .await
        }
    };
    match handler {
        Ok(_) => Ok(()),
        Err(_) => Err("Something went wrong in the Crawler".to_string()),
    }
}

/// Function to handle selenium.
/// Uses the chrome/chromium browser for now.
async fn handle_selenium(
    file_path: Option<PathBuf>,
    flag: bool,
    mut rx: mpsc::Receiver<String>,
) -> Result<(), thirtyfour::error::WebDriverError> {
    use thirtyfour::prelude::*;
    use tokio::fs;

    if flag {
        if let Some(file_path) = file_path {
            let mut caps = DesiredCapabilities::chrome();
            caps.add_chrome_arg("--enable-automation")?;
            let driver = WebDriver::new("http://localhost:4444/wd/hub", &caps).await?;
            // driver.fullscreen_window().await?;
            let mut file_name = 1;
            let mut file_path = file_path.clone();
            file_path.push("screenshots");
            if fs::create_dir_all(&file_path).await.is_err() {
                return Err(thirtyfour::error::WebDriverError::FatalError(
                    "".to_string(),
                ));
            }

            while let Some(link) = rx.recv().await {
                if driver.get(link.as_str()).await.is_ok() {
                    let mut img_path = file_path.clone();
                    img_path.push(file_name.to_string());
                    let _ = driver.fullscreen_window().await;
                    let _ = driver.screenshot(&img_path).await;
                    file_name += 1;
                }
            }
            let _ = driver.quit().await;
        }
    }
    Ok(())
}

/// Funtion to handle the output. Handles both console and file output.
/// Outputs in JSON format.
async fn handle_output(
    file_path: Option<PathBuf>,
    verbose: bool,
    mut rx: mpsc::Receiver<Link>,
    task_limit: usize,
) -> Result<(), String> {
    let mut senders = Vec::new();
    let mut handlers = Vec::new();
    if let Some(x) = file_path {
        let (tx, rx) = mpsc::channel::<Link>(task_limit);
        senders.push(tx);
        let handler = tokio::spawn(async move { file_handler::write_links(x, rx).await });
        handlers.push(handler);
    }
    if verbose {
        let (tx, rx) = mpsc::channel::<Link>(task_limit);
        senders.push(tx);
        let handler = tokio::spawn(async move { write_standard_output(rx).await });
        handlers.push(handler);
    }
    while let Some(link) = rx.recv().await {
        for i in &senders {
            if let Err(_) = i.send(link.clone()).await {
                let _ = futures::future::try_join_all(handlers).await;
                return Err("Something Wrong with IO".to_string());
            }
        }
    }

    senders.clear();

    let handle = futures::future::try_join_all(handlers).await;
    match handle {
        Err(x) => Err(x.to_string()),
        Ok(_) => Ok(()),
    }
}

/// Function to write to Standard Output.
async fn write_standard_output(mut rx: mpsc::Receiver<Link>) -> Result<(), std::io::Error> {
    while let Some(link) = rx.recv().await {
        println!("{},", link);
    }
    println!("");
    Ok(())
}
