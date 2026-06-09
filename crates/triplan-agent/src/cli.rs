use clap::{Args, Parser, Subcommand};
use serde_json::json;

use crate::db::{connect_sqlite, migrate, ClarificationStore};
use crate::error::Result;
use crate::history::ConversationHistoryStore;
use crate::tools::{ClarifyTool, ControlTool, ControlToolContext};

#[derive(Debug, Parser)]
#[command(
    name = "triplan-agent",
    version,
    about = "Ultra-lightweight, high-performance agent runtime kernel"
)]
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
        #[arg(long, default_value = "default")]
        conversation: String,
        prompt: Vec<String>,
    },
    Status,
    Events,
    Doctor,
    Compact {
        instructions: Vec<String>,
    },
    Clarify {
        #[command(subcommand)]
        command: ClarifyCommand,
    },
    History {
        #[command(subcommand)]
        command: HistoryCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ClarifyCommand {
    Request(ClarifyRequestArgs),
    List(ClarifyListArgs),
    Answer(ClarifyAnswerArgs),
}

#[derive(Debug, Subcommand)]
pub enum HistoryCommand {
    Add(HistoryAddArgs),
    List(HistoryListArgs),
    Recent(HistoryRecentArgs),
}

#[derive(Debug, Args)]
pub struct HistoryAddArgs {
    #[arg(long, default_value = "triplan-agent")]
    workspace: String,
    #[arg(long, default_value = "default")]
    conversation: String,
    #[arg(long, default_value = "user")]
    role: String,
    message: Vec<String>,
}

#[derive(Debug, Args)]
pub struct HistoryListArgs {
    #[arg(long, default_value_t = 20)]
    limit: usize,
}

#[derive(Debug, Args)]
pub struct HistoryRecentArgs {
    #[arg(long, default_value_t = 3)]
    limit: usize,
    #[arg(long, default_value_t = 12000)]
    max_bytes: usize,
}

#[derive(Debug, Args)]
pub struct ClarifyRequestArgs {
    #[arg(long)]
    conversation: String,
    #[arg(long)]
    run: String,
    #[arg(long, default_value = "default")]
    agent: String,
    #[arg(long)]
    question: String,
    #[arg(long)]
    reason: Option<String>,
    #[arg(long = "option")]
    options: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ClarifyListArgs {
    #[arg(long)]
    run: Option<String>,
}

#[derive(Debug, Args)]
pub struct ClarifyAnswerArgs {
    clarification_id: String,
    #[arg(long)]
    answer: String,
}

pub async fn dispatch(cli: Cli) -> Result<()> {
    match cli.command.unwrap_or(Command::Status) {
        Command::Init => {
            let paths = crate::config::runtime_paths()?;
            crate::config::init_environment_at(&paths).await?;
            let database_url = crate::config::checkpoint_database_url_for(&paths).await?;
            let pool = connect_sqlite(&database_url).await?;
            migrate(&pool).await?;
            pool.close().await;
            println!("initialized triplan-agent environment");
            println!("user_data: {}", paths.user_root_dir().display());
            println!("app_data: {}", paths.app_data_dir().display());
            println!(
                "checkpoint_db: {}",
                paths.checkpoint_database_path().display()
            );
        }
        Command::Chat { agent } => {
            println!("agent chat requested for {agent}");
        }
        Command::Run {
            agent,
            conversation,
            prompt,
        } => {
            let prompt = prompt.join(" ");
            println!("agent run requested for {agent}: {prompt}");
            if !prompt.trim().is_empty() {
                let paths = crate::config::runtime_paths()?;
                let config = crate::config::load_user_config_from(&paths).await?;
                let history = ConversationHistoryStore::new(&paths);
                let path = history
                    .append_message(&config.workspace_name, &conversation, "user", &prompt)
                    .await?;
                println!("history: {}", path.display());
            }
        }
        Command::Status => {
            let paths = crate::config::runtime_paths()?;
            let config = crate::config::load_user_config_from(&paths).await?;
            println!("workspace: {}", config.workspace_name);
            println!("default_agent: {}", config.default_agent);
            println!("default_provider: {}", config.default_provider);
            println!("user_data: {}", paths.user_root_dir().display());
            println!("app_data: {}", paths.app_data_dir().display());
            println!(
                "checkpoint_db: {}",
                paths.checkpoint_database_path().display()
            );
        }
        Command::Events => {
            println!("agent events: unavailable before init");
        }
        Command::Doctor => {
            println!("CLI: ok");
            println!("SQLite: stored in APP_DATA after triplan-agent init");
            println!("MCP: stdio transport planned");
            println!("User data: stored under ~/.triplan-agent/.agents by default");
            println!("Shadowbox: soft sandbox policy enabled from user config after init");
        }
        Command::Compact { instructions } => {
            let joined = instructions.join(" ");
            let summary = crate::context::CompactionEngine::summarize_for_test(&[&joined]);
            println!("{summary}");
        }
        Command::Clarify { command } => {
            dispatch_clarify(command).await?;
        }
        Command::History { command } => {
            dispatch_history(command).await?;
        }
    }
    Ok(())
}

async fn dispatch_history(command: HistoryCommand) -> Result<()> {
    let paths = crate::config::runtime_paths()?;
    let store = ConversationHistoryStore::new(&paths);
    match command {
        HistoryCommand::Add(args) => {
            let message = args.message.join(" ");
            let path = store
                .append_message(&args.workspace, &args.conversation, &args.role, &message)
                .await?;
            println!("history: {}", path.display());
        }
        HistoryCommand::List(args) => {
            let entries = store.recent_entries(args.limit)?;
            if entries.is_empty() {
                println!("no conversation history");
            } else {
                for entry in entries {
                    println!("{}", entry.path.display());
                }
            }
        }
        HistoryCommand::Recent(args) => {
            let context = store.recent_context(args.limit, args.max_bytes).await?;
            print!("{context}");
        }
    }
    Ok(())
}

async fn dispatch_clarify(command: ClarifyCommand) -> Result<()> {
    let (workspace_id, store) = open_workspace_clarifications().await?;
    match command {
        ClarifyCommand::Request(args) => {
            let tool = ClarifyTool::new(store);
            let output = tool
                .call(
                    ControlToolContext {
                        workspace_id: &workspace_id,
                        conversation_id: &args.conversation,
                        run_id: &args.run,
                        agent_id: &args.agent,
                    },
                    json!({
                        "question": args.question,
                        "reason": args.reason,
                        "options": args.options,
                    }),
                )
                .await?;
            println!(
                "clarification_id: {}",
                output["clarification_id"].as_str().unwrap_or("")
            );
            println!("status: {}", output["status"].as_str().unwrap_or(""));
            println!(
                "run_status: {}",
                output["run_status"].as_str().unwrap_or("")
            );
        }
        ClarifyCommand::List(args) => {
            let pending = store
                .list_pending(&workspace_id, args.run.as_deref())
                .await?;
            if pending.is_empty() {
                println!("no pending clarifications");
            } else {
                for item in pending {
                    println!(
                        "{}\t{}\t{}\t{}",
                        item.clarification_id, item.run_id, item.agent_id, item.question
                    );
                }
            }
        }
        ClarifyCommand::Answer(args) => {
            let answered = store.answer(&args.clarification_id, &args.answer).await?;
            println!("clarification_id: {}", answered.clarification_id);
            println!("status: {}", answered.status);
            println!("run_status: resumed");
        }
    }
    Ok(())
}

async fn open_workspace_clarifications() -> Result<(String, ClarificationStore)> {
    let config = crate::config::load_user_config().await?;
    let database_url = crate::config::checkpoint_database_url().await?;
    let pool = connect_sqlite(&database_url).await?;
    migrate(&pool).await?;
    Ok((config.workspace_name, ClarificationStore::new(pool)))
}
