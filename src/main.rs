use clap::Parser;

use agent_ease::cli::{dispatch, Cli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    dispatch(Cli::parse()).await?;
    Ok(())
}
