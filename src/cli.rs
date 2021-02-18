use clap::Clap;
use std::time::Instant;

#[derive(Clap)]
#[clap(version = "1.0", author = "Ayush Singh <ayushsingh1325@gmail.com>")]
struct CLI {
    url: String,
    #[clap(short, long, default_value = "0")]
    depth: usize,
    #[clap(short, long, default_value = "1000")]
    task_limit: usize,
}

pub async fn entry() {
    let start_time = Instant::now();
    println!("Started");
    let opts = CLI::parse();

    let links = crate::crawler::crawl_host(opts.url, opts.depth, std::cmp::max(opts.task_limit, 1))
        .await
        .unwrap();
    links.iter().for_each(|x| println!("{}", x));

    println!(
        "Time Taken: {} milli seconds",
        start_time.elapsed().as_millis()
    );
}
