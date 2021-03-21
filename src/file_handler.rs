use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::sync::mpsc::Receiver;

pub async fn read_hosts(
    file_path: PathBuf,
) -> Result<HashSet<url::Host>, Box<dyn std::error::Error>> {
    use tokio::io::{AsyncBufReadExt, BufReader};
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

pub async fn write_links(
    file_path: PathBuf,
    mut rx: Receiver<crate::extractors::links::Link>,
) -> Result<(), std::io::Error> {
    use tokio::io::{AsyncWriteExt, BufWriter};

    let file = File::create(file_path).await?;
    let mut writer = BufWriter::new(file);
    writer.write(b"[\n").await?;

    if let Some(link) = rx.recv().await {
        //serde_json::to_writer_pretty(&file, &link)?;
        let temp = serde_json::to_vec_pretty(&link)?;
        writer.write(&temp).await?;
    }

    while let Some(link) = rx.recv().await {
        writer.write(b",\n").await?;
        let temp = serde_json::to_vec_pretty(&link)?;
        writer.write(&temp).await?;
    }

    writer.write(b"\n]").await?;
    writer.flush().await?;
    Ok(())
}
