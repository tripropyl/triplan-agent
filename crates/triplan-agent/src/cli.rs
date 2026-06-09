use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use serde_json::json;

use crate::db::{connect_sqlite, migrate, ClarificationStore};
use crate::error::{AgentError, Result};
use crate::history::ConversationHistoryStore;
use crate::provider::{
    LlmProvider, ModelMessage, ModelOutput, ModelRequest, ModelStreamEvent,
    OpenAiCompatibleProvider,
};
use crate::tools::{ClarifyTool, ControlTool, ControlToolContext};

pub async fn run_from_env() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    dispatch(parse_from_env()?).await?;
    Ok(())
}

fn parse_from_env() -> anyhow::Result<Cli> {
    let display_name = env::args()
        .next()
        .and_then(|arg| {
            PathBuf::from(arg)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "triplan".to_string());
    let display_name: &'static str = Box::leak(display_name.into_boxed_str());
    let matches = Cli::command().name(display_name).get_matches();
    Ok(Cli::from_arg_matches(&matches)?)
}

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Ultra-lightweight, high-performance agent runtime kernel"
)]
pub struct Cli {
    #[arg(short = 'p', long = "print", help = "Run one prompt non-interactively")]
    pub print: Option<String>,
    #[arg(
        long,
        help = "Provider name from ~/.triplan-agent/.agents/providers.toml"
    )]
    pub provider: Option<String>,
    #[arg(long, help = "Override the selected agent profile model")]
    pub model: Option<String>,
    #[arg(long, help = "Stream text deltas as they arrive")]
    pub stream: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Text,
        help = "Output mode: text, one final json object, or newline-delimited stream-json"
    )]
    pub output_format: OutputFormat,
    #[arg(
        value_name = "WORKSPACE",
        help = "Run the default agent in a workspace path, for example: triplan ."
    )]
    pub workspace: Option<PathBuf>,
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
        #[arg(long, default_value = "default", help = "Agent profile name")]
        agent: String,
        #[arg(long, default_value = "default", help = "Conversation history id")]
        conversation: String,
        #[arg(
            long,
            help = "Provider name from ~/.triplan-agent/.agents/providers.toml"
        )]
        provider: Option<String>,
        #[arg(long, help = "Override the selected agent profile model")]
        model: Option<String>,
        #[arg(long, help = "Stream text deltas as they arrive")]
        stream: bool,
        #[arg(
            long,
            value_enum,
            default_value_t = OutputFormat::Text,
            help = "Output mode: text, one final json object, or newline-delimited stream-json"
        )]
        output_format: OutputFormat,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    StreamJson,
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
    if let Some(prompt) = cli.print {
        return dispatch_run(RunArgs {
            agent: "default".to_string(),
            conversation: "default".to_string(),
            provider: cli.provider,
            model: cli.model,
            stream: cli.stream,
            output_format: cli.output_format,
            workspace: None,
            prompt: vec![prompt],
        })
        .await;
    }

    if let Some(workspace) = cli.workspace {
        return dispatch_run(RunArgs {
            agent: "default".to_string(),
            conversation: "default".to_string(),
            provider: cli.provider,
            model: cli.model,
            stream: cli.stream || cli.output_format == OutputFormat::Text,
            output_format: cli.output_format,
            workspace: Some(workspace),
            prompt: vec![DEFAULT_WORKSPACE_PROMPT.to_string()],
        })
        .await;
    }

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
            println!("user_config: {}", paths.user_config_dir().display());
            println!(
                "conversation_history: {}",
                paths.conversation_history_dir().display()
            );
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
            provider,
            model,
            stream,
            output_format,
            prompt,
        } => {
            dispatch_run(RunArgs {
                agent,
                conversation,
                provider,
                model,
                stream,
                output_format,
                workspace: None,
                prompt,
            })
            .await?;
        }
        Command::Status => {
            let paths = crate::config::runtime_paths()?;
            let config = crate::config::load_user_config_from(&paths).await?;
            println!("workspace: {}", config.workspace_name);
            println!("default_agent: {}", config.default_agent);
            println!("default_provider: {}", config.default_provider);
            println!("user_data: {}", paths.user_root_dir().display());
            println!("user_config: {}", paths.user_config_dir().display());
            println!(
                "conversation_history: {}",
                paths.conversation_history_dir().display()
            );
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
            println!("User config: stored under ~/.triplan-agent/.agents by default");
            println!(
                "Conversation history: stored under ~/.triplan-agent/conversations by default"
            );
            println!("SQLite: stored in APP_DATA/triplan-agent/checkpoints.sqlite3 after init");
            println!("System resources: stored under APP_DATA/triplan-agent/resources after init");
            println!("MCP: stdio transport planned");
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

struct RunArgs {
    agent: String,
    conversation: String,
    provider: Option<String>,
    model: Option<String>,
    stream: bool,
    output_format: OutputFormat,
    workspace: Option<PathBuf>,
    prompt: Vec<String>,
}

struct ResolvedProvider {
    name: String,
    model: String,
    provider: OpenAiCompatibleProvider,
}

enum ProviderResolution {
    Ready(ResolvedProvider),
    MissingCredential {
        provider_name: String,
        key_env: String,
        model: String,
    },
    MisconfiguredCredential {
        provider_name: String,
        suggested_key_env: String,
        model: String,
    },
}

async fn dispatch_run(args: RunArgs) -> Result<()> {
    if let Some(workspace) = &args.workspace {
        env::set_current_dir(workspace).map_err(|err| {
            AgentError::Config(format!(
                "could not enter workspace `{}`: {err}",
                workspace.display()
            ))
        })?;
    }

    let prompt = args.prompt.join(" ");
    let stream = args.stream || args.output_format == OutputFormat::StreamJson;
    let json_lines = args.output_format == OutputFormat::StreamJson;
    if args.stream && args.output_format == OutputFormat::Json {
        return Err(AgentError::Config(
            "--stream with --output-format json is ambiguous; use --output-format stream-json"
                .to_string(),
        ));
    }
    let paths = crate::config::runtime_paths()?;
    let (config, initialized) = load_or_init_user_environment(&paths).await?;

    let history_path = if prompt.trim().is_empty() {
        None
    } else {
        let history = ConversationHistoryStore::new(&paths);
        Some(
            history
                .append_message(&config.workspace_name, &args.conversation, "user", &prompt)
                .await?,
        )
    };

    match args.output_format {
        OutputFormat::Text => {
            if let Some(workspace) = &args.workspace {
                println!("workspace: {}", workspace.display());
            }
            if initialized {
                println!("initialized triplan-agent environment");
                println!("user_config: {}", paths.user_config_dir().display());
            }
            println!("agent run requested for {}: {prompt}", args.agent);
            if let Some(path) = &history_path {
                println!("history: {}", path.display());
            }
        }
        OutputFormat::Json => {}
        OutputFormat::StreamJson => {
            write_json_line(json!({
                "type": "run_started",
                "agent": &args.agent,
                "conversation": &args.conversation,
                "workspace": &config.workspace_name,
                "workspace_path": args.workspace.as_ref(),
            }))?;
            if initialized {
                write_json_line(json!({
                    "type": "environment_initialized",
                    "user_config": paths.user_config_dir(),
                    "app_data": paths.app_data_dir(),
                }))?;
            }
            if let Some(path) = &history_path {
                write_json_line(json!({
                    "type": "history_appended",
                    "role": "user",
                    "path": path,
                }))?;
            }
        }
    }

    if prompt.trim().is_empty() {
        if args.output_format == OutputFormat::Json {
            println!(
                "{}",
                serde_json::to_string(&json!({
                    "type": "run_completed",
                    "status": "empty_prompt",
                }))?
            );
        } else if json_lines {
            write_json_line(json!({
                "type": "run_completed",
                "status": "empty_prompt",
            }))?;
        }
        return Ok(());
    }

    let provider = resolve_provider(
        &paths,
        &config.default_provider,
        &args.agent,
        args.provider.as_deref(),
        args.model.as_deref(),
        false,
    )
    .await?;
    let resolved = match provider {
        ProviderResolution::Ready(provider) => provider,
        ProviderResolution::MissingCredential {
            provider_name,
            key_env,
            model,
        } => {
            let setup = credential_setup_message(&provider_name, &key_env, &model, &paths);
            if args.output_format == OutputFormat::Text {
                println!("{setup}");
                return Ok(());
            }
            if args.output_format == OutputFormat::StreamJson {
                write_json_line(json!({
                    "type": "error",
                    "code": "missing_credential",
                    "provider": provider_name,
                    "api_key_env": key_env,
                    "model": model,
                    "message": setup,
                }))?;
                write_json_line(json!({
                    "type": "run_completed",
                    "status": "missing_credential",
                }))?;
            } else {
                println!(
                    "{}",
                    serde_json::to_string(&json!({
                        "type": "run_completed",
                        "status": "missing_credential",
                        "provider": provider_name,
                        "api_key_env": key_env,
                        "model": model,
                        "message": setup,
                    }))?
                );
            }
            return Ok(());
        }
        ProviderResolution::MisconfiguredCredential {
            provider_name,
            suggested_key_env,
            model,
        } => {
            let setup =
                credential_config_error_message(&provider_name, &suggested_key_env, &model, &paths);
            if args.output_format == OutputFormat::Text {
                println!("{setup}");
            } else if args.output_format == OutputFormat::StreamJson {
                write_json_line(json!({
                    "type": "error",
                    "code": "misconfigured_credential",
                    "provider": provider_name,
                    "suggested_api_key_env": suggested_key_env,
                    "model": model,
                    "message": setup,
                }))?;
                write_json_line(json!({
                    "type": "run_completed",
                    "status": "misconfigured_credential",
                }))?;
            } else {
                println!(
                    "{}",
                    serde_json::to_string(&json!({
                        "type": "run_completed",
                        "status": "misconfigured_credential",
                        "provider": provider_name,
                        "suggested_api_key_env": suggested_key_env,
                        "model": model,
                        "message": setup,
                    }))?
                );
            }
            return Ok(());
        }
    };

    let request = model_request(&paths, &args.agent, &prompt, &resolved.model).await?;
    let output = if stream {
        stream_run_output(&resolved, request, args.output_format).await?
    } else {
        let output = resolved.provider.complete(request).await?;
        write_model_output(&resolved, &output, args.output_format)?;
        output
    };

    if let ModelOutput::Text(text) = &output {
        if !text.trim().is_empty() {
            let history = ConversationHistoryStore::new(&paths);
            let path = history
                .append_message(
                    &config.workspace_name,
                    &args.conversation,
                    "assistant",
                    text,
                )
                .await?;
            if args.output_format == OutputFormat::StreamJson {
                write_json_line(json!({
                    "type": "history_appended",
                    "role": "assistant",
                    "path": path,
                }))?;
            }
        }
    }

    if args.output_format == OutputFormat::StreamJson {
        write_json_line(json!({
            "type": "run_completed",
            "provider": resolved.name,
            "model": resolved.model,
            "output": output,
        }))?;
    }

    Ok(())
}

async fn resolve_provider(
    paths: &crate::config::RuntimePaths,
    default_provider: &str,
    agent_name: &str,
    provider_override: Option<&str>,
    model_override: Option<&str>,
    require_credentials: bool,
) -> Result<ProviderResolution> {
    load_local_env();
    let agent = crate::config::load_agent_profile_from(paths, agent_name).await?;
    let provider_name = provider_override.unwrap_or(default_provider).to_string();
    let endpoint = crate::config::load_provider_endpoint_from(paths, &provider_name).await?;
    let model = model_override.unwrap_or(&agent.model).to_string();
    let key_env = endpoint.api_key_env.trim().to_string();
    if looks_like_api_key_value(&key_env) {
        return Ok(ProviderResolution::MisconfiguredCredential {
            suggested_key_env: suggested_api_key_env(&provider_name),
            provider_name,
            model,
        });
    }

    let api_key = match env::var(&key_env) {
        Ok(value) if !value.trim().is_empty() => value,
        _ if require_credentials => {
            return Err(AgentError::Config(format!(
                "missing {} for provider `{provider_name}`",
                key_env
            )));
        }
        _ => {
            return Ok(ProviderResolution::MissingCredential {
                provider_name,
                key_env,
                model,
            });
        }
    };
    Ok(ProviderResolution::Ready(ResolvedProvider {
        name: provider_name.clone(),
        model: model.clone(),
        provider: OpenAiCompatibleProvider::new(provider_name, endpoint.base_url, api_key, model),
    }))
}

fn looks_like_api_key_value(value: &str) -> bool {
    let value = value.trim();
    value.starts_with("sk-")
        || value.starts_with("sk_")
        || value.starts_with("Bearer ")
        || value.starts_with("bailian-")
        || value.len() > 48
            && value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

fn suggested_api_key_env(provider_name: &str) -> String {
    match provider_name {
        "dashscope" => "DASHSCOPE_API_KEY".to_string(),
        "openai" => "OPENAI_API_KEY".to_string(),
        other => {
            let mut name = String::new();
            for ch in other.chars() {
                if ch.is_ascii_alphanumeric() {
                    name.push(ch.to_ascii_uppercase());
                } else {
                    name.push('_');
                }
            }
            if name.is_empty() {
                "LLM_API_KEY".to_string()
            } else {
                format!("{name}_API_KEY")
            }
        }
    }
}

async fn load_or_init_user_environment(
    paths: &crate::config::RuntimePaths,
) -> Result<(crate::config::WorkspaceConfig, bool)> {
    let needs_init = !paths.user_config_path().is_file()
        || !paths.providers_path().is_file()
        || !paths.agent_profiles_dir().join("default.toml").is_file();
    if needs_init {
        crate::config::init_environment_at(paths).await?;
        let database_url = crate::config::checkpoint_database_url_for(paths).await?;
        let pool = connect_sqlite(&database_url).await?;
        migrate(&pool).await?;
        pool.close().await;
        let config = crate::config::load_user_config_from(paths).await?;
        return Ok((config, true));
    }

    Ok((crate::config::load_user_config_from(paths).await?, false))
}

fn credential_setup_message(
    provider_name: &str,
    key_env: &str,
    model: &str,
    paths: &crate::config::RuntimePaths,
) -> String {
    format!(
        "missing API key for provider `{provider_name}`\n\
         expected env var: {key_env}\n\
         model: {model}\n\
         set it for this shell:\n\
           export {key_env}=...\n\
         or add `{key_env}=...` to a .env file in your workspace\n\
         provider config: {}",
        paths.providers_path().display()
    )
}

fn credential_config_error_message(
    provider_name: &str,
    suggested_key_env: &str,
    model: &str,
    paths: &crate::config::RuntimePaths,
) -> String {
    format!(
        "provider `{provider_name}` credential config looks wrong\n\
         `api_key_env` should be an environment variable name, not the API key value\n\
         model: {model}\n\
         edit provider config:\n\
           {}\n\
         set:\n\
           api_key_env = \"{suggested_key_env}\"\n\
         then put the real key in this shell or workspace .env:\n\
           export {suggested_key_env}=...",
        paths.providers_path().display()
    )
}

async fn model_request(
    paths: &crate::config::RuntimePaths,
    agent_name: &str,
    prompt: &str,
    model: &str,
) -> Result<ModelRequest> {
    let agent = crate::config::load_agent_profile_from(paths, agent_name).await?;
    let mut messages = Vec::new();
    if !agent.system_prompt.trim().is_empty() {
        messages.push(ModelMessage {
            role: "system".to_string(),
            content: agent.system_prompt,
        });
    }
    messages.push(ModelMessage {
        role: "user".to_string(),
        content: prompt.to_string(),
    });
    Ok(ModelRequest {
        model: model.to_string(),
        messages,
    })
}

async fn stream_run_output(
    provider: &ResolvedProvider,
    request: ModelRequest,
    output_format: OutputFormat,
) -> Result<ModelOutput> {
    match output_format {
        OutputFormat::StreamJson => {
            let mut sink = |event: ModelStreamEvent| -> Result<()> {
                match event {
                    ModelStreamEvent::ContentDelta { delta } => {
                        write_json_line(json!({
                            "type": "content_delta",
                            "delta": delta,
                        }))?;
                    }
                    ModelStreamEvent::ToolCallDelta { name, arguments } => {
                        write_json_line(json!({
                            "type": "tool_call_delta",
                            "name": name,
                            "arguments": arguments,
                        }))?;
                    }
                }
                Ok(())
            };
            provider.provider.stream(request, &mut sink).await
        }
        OutputFormat::Text | OutputFormat::Json => {
            let mut sink = |event: ModelStreamEvent| -> Result<()> {
                if output_format == OutputFormat::Text {
                    let ModelStreamEvent::ContentDelta { delta } = event else {
                        return Ok(());
                    };
                    print!("{delta}");
                    io::stdout().flush()?;
                }
                Ok(())
            };
            let output = provider.provider.stream(request, &mut sink).await?;
            if output_format == OutputFormat::Text {
                println!();
            }
            if output_format == OutputFormat::Json {
                write_model_output(provider, &output, output_format)?;
            }
            Ok(output)
        }
    }
}

fn write_model_output(
    provider: &ResolvedProvider,
    output: &ModelOutput,
    output_format: OutputFormat,
) -> Result<()> {
    match output_format {
        OutputFormat::Text => match output {
            ModelOutput::Text(text) => println!("{text}"),
            ModelOutput::ToolCall { name, arguments } => {
                println!("tool_call: {name} {arguments}");
            }
        },
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string(&json!({
                    "type": "run_completed",
                    "provider": provider.name,
                    "model": provider.model,
                    "output": output,
                }))?
            );
        }
        OutputFormat::StreamJson => {}
    }
    Ok(())
}

fn write_json_line(value: serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string(&value)?);
    io::stdout().flush()?;
    Ok(())
}

fn load_local_env() {
    let _ = dotenvy::dotenv();
    for path in ancestor_env_files() {
        let _ = dotenvy::from_path(path);
    }
}

fn ancestor_env_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        for ancestor in cwd.ancestors() {
            let path = ancestor.join(".env");
            if path.is_file() {
                files.push(path);
            }
        }
    }
    files
}

const DEFAULT_WORKSPACE_PROMPT: &str =
    "Inspect this workspace and summarize what is present, then suggest the next useful action.";

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
