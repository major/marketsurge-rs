#[cfg(not(coverage))]
#[tokio::main]
async fn main() {
    std::process::exit(marketsurge_agent::run().await);
}

#[cfg(coverage)]
fn main() {}
