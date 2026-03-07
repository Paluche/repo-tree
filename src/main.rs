#[tokio::main]
async fn main() {
    std::process::exit(repo_tree::cli::run().await);
}
