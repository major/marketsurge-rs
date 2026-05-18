#[tokio::main]
async fn main() {
    std::process::exit(marketsurge_agent::run().await);
}
