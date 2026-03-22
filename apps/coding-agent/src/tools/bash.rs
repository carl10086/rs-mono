use crate::agent::{Tool, ToolContext, ToolResult};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

const DEFAULT_TIMEOUT_MS: u64 = 120_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashToolArgs {
    pub command: String,
    pub description: String,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub workdir: Option<String>,
}

pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }

    async fn run_command(
        command: &str,
        workdir: Option<&str>,
        timeout_ms: u64,
    ) -> Result<(String, Option<bool>, i32)> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }

        let child = cmd.spawn()?;

        let timeout_duration = Duration::from_millis(timeout_ms);

        let result = timeout(timeout_duration, async {
            let mut stdout_buf = String::new();
            let mut stderr_buf = String::new();

            let output = child.wait_with_output().await?;

            if !output.stdout.is_empty() {
                stdout_buf = String::from_utf8_lossy(&output.stdout).to_string();
            }
            if !output.stderr.is_empty() {
                stderr_buf = String::from_utf8_lossy(&output.stderr).to_string();
            }

            Ok::<_, anyhow::Error>((stdout_buf, stderr_buf, output.status.code().unwrap_or(-1)))
        })
        .await;

        match result {
            Ok(Ok((stdout, stderr, exit_code))) => {
                let combined = if stderr.is_empty() {
                    stdout
                } else {
                    format!("{}{}", stdout, stderr)
                };
                Ok((combined, None, exit_code))
            }
            Ok(Err(e)) => Err(anyhow!("Command execution error: {}", e)),
            Err(_) => Ok((String::new(), Some(true), -1)),
        }
    }
}

impl Tool for BashTool {
    fn define(&self) -> crate::agent::types::ToolDefine {
        crate::agent::types::ToolDefine {
            name: "bash".to_string(),
            description: include_str!("bash.txt").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    },
                    "description": {
                        "type": "string",
                        "description": "A brief description of the command (5-10 words)"
                    },
                    "timeout": {
                        "type": "number",
                        "description": "Optional timeout in milliseconds (default 120000)"
                    },
                    "workdir": {
                        "type": "string",
                        "description": "Optional working directory"
                    }
                },
                "required": ["command", "description"]
            }),
        }
    }

    fn execute(
        &self,
        args: serde_json::Value,
        _ctx: ToolContext,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult>> + Send + '_>> {
        let args: Result<BashToolArgs, _> = serde_json::from_value(args);

        Box::pin(async move {
            let args = args.map_err(|e| anyhow!("Invalid args: {}", e))?;

            let timeout_ms = args.timeout.unwrap_or(DEFAULT_TIMEOUT_MS);
            let workdir = args.workdir.as_deref();

            let (output, timed_out, exit_code) =
                Self::run_command(&args.command, workdir, timeout_ms).await?;

            let mut metadata = serde_json::json!({
                "exit_code": exit_code,
            });

            if timed_out.unwrap_or(false) {
                metadata["timed_out"] = serde_json::json!(true);
            }

            Ok(ToolResult::new(args.description.clone(), output).with_metadata(metadata))
        })
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}