use clap::{Parser, Subcommand};

use crate::error::Result;

#[derive(Debug, Parser)]
#[command(name = "agent", version, about = "Local event-driven agent runtime")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init,
    Chat {
        #[arg(long, default_value = "default")]
        agent: String,
    },
    Run {
        #[arg(long, default_value = "default")]
        agent: String,
        prompt: Vec<String>,
    },
    Status,
    Events,
    Doctor,
    Compact {
        instructions: Vec<String>,
    },
}

pub async fn dispatch(cli: Cli) -> Result<()> {
    match cli.command.unwrap_or(Command::Status) {
        Command::Init => {
            crate::config::init_workspace(&std::env::current_dir()?).await?;
            println!("initialized agent workspace");
        }
        Command::Chat { agent } => {
            println!("agent chat requested for {agent}");
        }
        Command::Run { agent, prompt } => {
            println!("agent run requested for {agent}: {}", prompt.join(" "));
        }
        Command::Status => {
            let cwd = std::env::current_dir()?;
            let config = crate::config::load_workspace_config(&cwd).await?;
            println!("workspace: {}", config.workspace_name);
            println!("default_agent: {}", config.default_agent);
            println!("default_provider: {}", config.default_provider);
        }
        Command::Events => {
            println!("agent events: unavailable before init");
        }
        Command::Doctor => {
            println!("CLI: ok");
            println!("SQLite: configured after agent init");
            println!("MCP: stdio transport planned");
            println!("Shadowbox: soft sandbox policy enabled after init");
        }
        Command::Compact { instructions } => {
            let joined = instructions.join(" ");
            let summary = crate::context::CompactionEngine::summarize_for_test(&[&joined]);
            println!("{summary}");
        }
    }
    Ok(())
}
