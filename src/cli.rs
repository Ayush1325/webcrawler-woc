use crate::extractors::links::Link;
use crate::file_handler;
use clap::Clap;
use std::{collections::HashSet, path::PathBuf, time::Instant};
use tokio::sync::{mpsc, mpsc::UnboundedReceiver};

#[derive(Clap)]
#[clap(version = "1.0", author = "Ayush Singh <ayushsingh1325@gmail.com>")]
struct CLI {
    url: String,
    #[clap(short, long)]
    depth: Option<usize>,
    #[clap(short, long, default_value = "1000")]
    task_limit: usize,
    #[clap(short, long)]
    whitelist: Option<PathBuf>,
    #[clap(short, long)]
    blacklist: Option<PathBuf>,
    #[clap(short, long)]
    output: Option<PathBuf>,
    #[clap(long)]
    verbose: bool,
}

pub async fn entry() {
    let start_time = Instant::now();
    println!("Started");
    let opts = CLI::parse();
    let (tx, rx) = mpsc::unbounded_channel();

    let output_handler = handle_output(opts.output, opts.verbose, rx);
    let crawler_handler = launch_crawler(opts.url, opts.depth, opts.task_limit, tx);

    let returns = futures::future::try_join(output_handler, crawler_handler).await;

    if let Err(x) = returns {
        println!("Error : {}", x);
    }

    println!(
        "Time Taken: {} milli seconds",
        start_time.elapsed().as_millis()
    );
}

async fn launch_crawler(
    origin_url: String,
    depth: Option<usize>,
    task_limit: usize,
    tx: mpsc::UnboundedSender<Link>,
) -> Result<(), String> {
    let origin_url = match Link::new(origin_url.as_str()) {
        Some(x) => x,
        None => return Err("Invalid Url".to_string()),
    };

    let handler = tokio::spawn(async move {
        crate::crawler::crawl(origin_url, depth, temp_whitellist(), None, tx, task_limit).await
    })
    .await;
    match handler {
        Ok(_) => Ok(()),
        Err(_) => Err("Something went wrong in the Crawler".to_string()),
    }
}

fn temp_whitellist() -> Option<HashSet<String>> {
    let mut temp = HashSet::new();
    temp.insert("crawler-test.com".to_string());
    Some(temp)
}

async fn handle_output(
    file_path: Option<PathBuf>,
    verbose: bool,
    mut rx: mpsc::UnboundedReceiver<Link>,
) -> Result<(), String> {
    let mut senders = Vec::new();
    let mut handlers = Vec::new();
    if let Some(x) = file_path {
        let (tx, rx) = mpsc::unbounded_channel::<Link>();
        senders.push(tx);
        let handler = tokio::spawn(async move { file_handler::write_links(x, rx).await });
        handlers.push(handler);
    }
    if verbose {
        let (tx, rx) = mpsc::unbounded_channel::<Link>();
        senders.push(tx);
        let handler = tokio::spawn(async move { write_standard_output(rx).await });
        handlers.push(handler);
    }
    while let Some(link) = rx.recv().await {
        senders.iter().for_each(|x| {
            x.send(link.clone());
        });
    }

    senders.clear();

    let handle = futures::future::try_join_all(handlers).await;
    match handle {
        Err(x) => Err(x.to_string()),
        Ok(_) => Ok(()),
    }
}

async fn write_standard_output(mut rx: UnboundedReceiver<Link>) -> Result<(), std::io::Error> {
    while let Some(link) = rx.recv().await {
        let json = serde_json::to_string_pretty(&link)?;
        println!("{},", json);
    }
    println!("");
    Ok(())
}
