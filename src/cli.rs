use crate::extractors::links::Link;
use crate::file_handler;
use clap::Clap;
use std::{collections::HashSet, path::PathBuf, time::Instant};
use tokio::sync::mpsc;

#[derive(Clap)]
#[clap(version = "1.0", author = "Ayush Singh <ayushsingh1325@gmail.com>")]
struct CLI {
    /// Url to be crawled.
    url: String,
    /// Gives numeric depth for crawl.
    #[clap(short, long)]
    depth: Option<usize>,
    /// Limits the number of parallel tasks. Doesn't work yet.
    #[clap(long, default_value = "1000")]
    task_limit: usize,
    /// Path of file containing list of domains to be crawled.
    #[clap(short, long)]
    whitelist: Option<PathBuf>,
    /// Path of file containing list of domains not to be crawled.
    #[clap(short, long)]
    blacklist: Option<PathBuf>,
    /// Path to file containing words to search for
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
}

pub async fn entry() {
    let start_time = Instant::now();
    println!("Started");
    let opts = CLI::parse();
    let (tx, rx) = mpsc::channel(opts.task_limit);

    let output_handler = handle_output(opts.output_folder, opts.verbose, rx, opts.task_limit);
    let crawler_handler = launch_crawler(
        opts.url,
        opts.depth,
        opts.task_limit,
        tx,
        opts.whitelist,
        opts.blacklist,
        opts.search_words,
        opts.timeout,
    );

    let returns = futures::future::try_join(output_handler, crawler_handler).await;

    if let Err(x) = returns {
        println!("Error : {}", x);
    }

    println!("Time Taken: {} seconds", start_time.elapsed().as_secs());
}

async fn launch_crawler(
    origin_url: String,
    depth: Option<usize>,
    task_limit: usize,
    tx: mpsc::Sender<Link>,
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
        None => tokio::spawn(async move {
            crate::crawler::crawl_no_depth(
                origin_url, whitelist, blacklist, word_list, tx, task_limit, timeout,
            )
            .await
        }),
        Some(x) => tokio::spawn(async move {
            crate::crawler::crawl_with_depth(
                origin_url, x, whitelist, blacklist, word_list, tx, task_limit, timeout,
            )
            .await
        }),
    };
    match handler.await {
        Ok(_) => Ok(()),
        Err(_) => Err("Something went wrong in the Crawler".to_string()),
    }
}

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

async fn write_standard_output(mut rx: mpsc::Receiver<Link>) -> Result<(), std::io::Error> {
    while let Some(link) = rx.recv().await {
        let json = serde_json::to_string_pretty(&link)?;
        println!("{},", json);
    }
    println!("");
    Ok(())
}
