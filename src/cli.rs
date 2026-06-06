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
}

pub async fn dispatch(cli: Cli) -> Result<()> {
    match cli.command.unwrap_or(Command::Status) {
        Command::Init => {
            println!("agent init is not implemented yet");
        }
        Command::Chat { agent } => {
            println!("agent chat requested for {agent}");
        }
        Command::Run { agent, prompt } => {
            println!("agent run requested for {agent}: {}", prompt.join(" "));
        }
        Command::Status => {
            println!("agent runtime status: uninitialized");
        }
        Command::Events => {
            println!("agent events: unavailable before init");
        }
        Command::Doctor => {
            println!("agent doctor: basic CLI is available");
        }
    }
    Ok(())
}
