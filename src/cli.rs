use clap::Clap;
use std::{path::PathBuf, time::Instant};

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
}

pub async fn entry() {
    let start_time = Instant::now();
    println!("Started");
    let opts = CLI::parse();

    if let Err(x) = lauch_crawler(opts).await {
        println!("Error: {}", x);
    }

    println!(
        "Time Taken: {} milli seconds",
        start_time.elapsed().as_millis()
    );
}

async fn lauch_crawler(opts: CLI) -> Result<(), String> {
    use crate::extractors::links::Link;

    let origin_url = match Link::new(opts.url.as_str()) {
        Some(x) => x,
        None => return Err("Invalid Url".to_string()),
    };
    let links = crate::crawler::crawl(origin_url, opts.depth, None, None)
        .await
        .unwrap();
    println!("Links Found: {}", links.len());
    Ok(())
}
