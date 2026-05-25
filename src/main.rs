#[tokio::main]
async fn main() {
    let code = rusty_marketsurge::cli::run().await;
    std::process::exit(code);
}
