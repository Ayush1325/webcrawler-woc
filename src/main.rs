mod cli;
mod crawler;
mod extractors;
mod file_handler;

#[tokio::main]
async fn main() {
    cli::entry().await;
    //test().await;
}

async fn test() {
    use std::time::Instant;
    use trust_dns_resolver::TokioAsyncResolver;

    let resolver = TokioAsyncResolver::tokio_from_system_conf().unwrap();

    let response = resolver.ipv4_lookup("www.google.com.").await.unwrap();

    //println!("{:#?}", response.valid_until());
    println!("{:#?}", response.iter().next());
}
