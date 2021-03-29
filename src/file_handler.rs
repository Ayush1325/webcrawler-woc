use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::mpsc::Receiver;

pub async fn read_hosts(
    file_path: PathBuf,
) -> Result<HashSet<url::Host>, Box<dyn std::error::Error>> {
    use url::Host;

    let file = File::open(file_path).await?;
    let reader = BufReader::new(file);
    let mut list = reader.lines();
    let mut hosts = HashSet::new();
    while let Ok(Some(x)) = list.next_line().await {
        if let Ok(host) = Host::parse(&x) {
            hosts.insert(host);
        }
    }

    Ok(hosts)
}

pub async fn read_words(file_path: PathBuf) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let file = File::open(file_path).await?;
    let reader = BufReader::new(file);
    let mut list = reader.lines();
    let mut words = HashSet::new();
    while let Ok(Some(x)) = list.next_line().await {
        words.insert(x);
    }

    Ok(words)
}

pub async fn write_links(
    folder_path: PathBuf,
    mut rx: Receiver<crate::extractors::links::Link>,
) -> Result<(), std::io::Error> {
    const CRAWLED_FILE_NAME: &str = r#"crawled.txt"#;
    const NOT_CRAWLED_FILE_NAME: &str = r#"not_crawled.txt"#;

    let mut crawled_file_path = folder_path.clone();
    crawled_file_path.push(CRAWLED_FILE_NAME);
    let crawled_file = File::create(crawled_file_path).await?;
    let mut crawled_writer = BufWriter::new(crawled_file);
    crawled_writer.write(b"[\n").await?;

    let mut not_crawled_file_path = folder_path.clone();
    not_crawled_file_path.push(NOT_CRAWLED_FILE_NAME);
    let not_crawled_file = File::create(not_crawled_file_path).await?;
    let mut not_crawled_writer = BufWriter::new(not_crawled_file);
    not_crawled_writer.write(b"[\n").await?;

    while let Some(link) = rx.recv().await {
        let temp = serde_json::to_vec_pretty(&link)?;
        if link.crawled {
            crawled_writer.write(&temp).await?;
            crawled_writer.write(b",\n").await?;
        } else {
            not_crawled_writer.write(&temp).await?;
            not_crawled_writer.write(b",\n").await?;
        }
    }

    crawled_writer.write(b"{}\n]").await?;
    crawled_writer.flush().await?;

    not_crawled_writer.write(b"{}\n]").await?;
    not_crawled_writer.flush().await?;

    Ok(())
}
