use clap::Clap;
use std::time::Instant;

#[derive(Clap)]
#[clap(version = "1.0", author = "Ayush Singh <ayushsingh1325@gmail.com>")]
struct CLI {
    url: String,
    #[clap(short, long)]
    depth: Option<usize>,
}

pub enum CrawlDepth {
    Page,
    Variable(usize),
    Domain,
}

impl CrawlDepth {
    fn from_option(depth: Option<usize>) -> Self {
        match depth {
            Some(x) => match x {
                0 => CrawlDepth::Domain,
                _ => CrawlDepth::Variable(x),
            },
            None => CrawlDepth::Page,
        }
    }
}

pub async fn entry() {
    let start_time = Instant::now();
    println!("Started");
    let opts = CLI::parse();

    let links = crate::crawler::crawl_host(opts.url, CrawlDepth::from_option(opts.depth))
        .await
        .unwrap();
    links.iter().for_each(|x| println!("{}", x));

    println!(
        "Time Taken: {} milli seconds",
        start_time.elapsed().as_millis()
    );
}
