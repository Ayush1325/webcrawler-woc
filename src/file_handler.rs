use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::mpsc::Receiver;

use crate::extractors::links;

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
    mut rx: Receiver<links::Link>,
) -> Result<(), std::io::Error> {
    const CRAWLED_FILE_NAME: &str = r#"crawled.json"#;
    const NOT_CRAWLED_FILE_NAME: &str = r#"not_crawled.json"#;
    const MAIL_FILE_NAME: &str = r#"emails.json"#;
    const TEL_FILE_NAME: &str = r#"phone_nos.json"#;

    let mut crawled_writer = init_writer(CRAWLED_FILE_NAME, &folder_path).await?;
    let mut not_crawled_writer = init_writer(NOT_CRAWLED_FILE_NAME, &folder_path).await?;
    let mut mail_writer = init_writer(MAIL_FILE_NAME, &folder_path).await?;
    let mut tel_writer = init_writer(TEL_FILE_NAME, &folder_path).await?;

    while let Some(link) = rx.recv().await {
        let temp = serde_json::to_vec_pretty(&link)?;
        match link.link_type {
            links::LinkType::Mail => write_json(&mut mail_writer, &temp).await?,
            links::LinkType::PhoneNo => write_json(&mut tel_writer, &temp).await?,
            links::LinkType::Other => {
                if link.crawled {
                    write_json(&mut crawled_writer, &temp).await?;
                } else {
                    write_json(&mut not_crawled_writer, &temp).await?;
                }
            }
        };
    }

    clean_writer(&mut crawled_writer).await?;
    clean_writer(&mut not_crawled_writer).await?;
    clean_writer(&mut mail_writer).await?;
    clean_writer(&mut tel_writer).await?;

    Ok(())
}

async fn init_writer(
    file_name: &str,
    folder_path: &PathBuf,
) -> Result<BufWriter<File>, std::io::Error> {
    let mut file_path = folder_path.clone();
    file_path.push(file_name);
    let mut writer = BufWriter::new(File::create(file_path).await?);
    writer.write(b"[\n").await?;
    Ok(writer)
}

async fn write_json(writer: &mut BufWriter<File>, json: &[u8]) -> Result<(), std::io::Error> {
    writer.write(json).await?;
    writer.write(b",\n").await?;
    Ok(())
}

async fn clean_writer(writer: &mut BufWriter<File>) -> Result<(), std::io::Error> {
    writer.write(b"{}\n]").await?;
    writer.flush().await?;
    Ok(())
}
