mod retriever;

#[tokio::main]
async fn main() {
    retriever::temp().await;
}
