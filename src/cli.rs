use clap::Clap;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clap)]
#[clap(version = "1.0", author = "Ayush Singh <ayushsingh1325@gmail.com>")]
struct CLI {
    url: String,
    #[clap(short, long)]
    depth: Option<usize>,
    #[clap(short, long, default_value = "1000")]
    task_limit: usize,
    #[clap(short, long)]
    blacklist: Option<PathBuf>,
    #[clap(short, long)]
    whitelist: Option<PathBuf>,
}

pub async fn entry() {
    let start_time = Instant::now();
    println!("Started");
    let opts = CLI::parse();

    let links = crate::crawler::crawl(opts.url, opts.depth).await.unwrap();
    links.iter().for_each(|x| println!("{:#?}", x));
    println!("Links Found: {}", links.len());

    println!(
        "Time Taken: {} milli seconds",
        start_time.elapsed().as_millis()
    );
}
