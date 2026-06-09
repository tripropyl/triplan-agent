use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::time::{timeout, Duration};

use crate::error::{AgentError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Value,
}

impl McpRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

pub struct StdioMcpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    request_timeout: Duration,
}

impl StdioMcpClient {
    pub async fn spawn(command: &str, args: &[String]) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AgentError::Runtime("mcp child stdin unavailable".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AgentError::Runtime("mcp child stdout unavailable".to_string()))?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            request_timeout: Duration::from_secs(10),
        })
    }

    pub async fn request(&mut self, request: McpRequest) -> Result<Value> {
        let mut line = serde_json::to_string(&request)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;

        let mut response = String::new();
        let bytes_read = timeout(self.request_timeout, self.stdout.read_line(&mut response))
            .await
            .map_err(|_| AgentError::Runtime("mcp request timed out".to_string()))??;
        if bytes_read == 0 {
            return Err(AgentError::Runtime(
                "mcp child closed stdout before response".to_string(),
            ));
        }
        Ok(serde_json::from_str(&response)?)
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.stdin.shutdown().await?;
        match timeout(Duration::from_secs(2), self.child.wait()).await {
            Ok(result) => {
                result?;
            }
            Err(_) => {
                self.child.kill().await?;
            }
        }
        Ok(())
    }
}
