#[tokio::main]
async fn main() {
    std::process::exit(repo_tree::run().await);
}
