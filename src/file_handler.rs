use std::collections::HashSet;
use std::path::PathBuf;
use tokio::sync::mpsc::Receiver;

pub fn read_urls(file_path: PathBuf) -> Result<HashSet<String>, std::io::Error> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let list = reader
        .lines()
        .filter_map(|x| match x {
            Ok(x) => Some(x),
            Err(_) => None,
        })
        .collect();
    Ok(list)
}

pub async fn write_links(
    file_path: PathBuf,
    mut rx: Receiver<crate::extractors::links::Link>,
) -> Result<(), std::io::Error> {
    use tokio::fs::File;
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
