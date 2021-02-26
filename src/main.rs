mod cli;
mod crawler;
mod extractors;
mod file_handler;

#[tokio::main]
async fn main() {
    cli::entry().await;
    //test().await;
}

// #[cfg(debug_assertions)]
// async fn test() {
//     let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
//     let file_path =
//         std::path::PathBuf::from(r"/home/ayush/Documents/Programming/Projects/WOC/temp.json");
//     let tx2 = tx.clone();
//     tokio::spawn(async move {
//         let link =
//             extractors::links::Link::new("https://tokio.rs/tokio/tutorial/channels").unwrap();
//         tx.send(link).unwrap();
//     });

//     tokio::spawn(async move {
//         let link = extractors::links::Link::new("https://serde.rs/impl-serialize.html").unwrap();
//         tx2.send(link).unwrap();
//     });

//     tokio::spawn(file_handler::write_links(file_path, rx)).await;
// }
