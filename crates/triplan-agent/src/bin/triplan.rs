#[tokio::main]
async fn main() -> anyhow::Result<()> {
    triplan_agent::cli::run_from_env().await
}
