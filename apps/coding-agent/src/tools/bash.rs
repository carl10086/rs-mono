//! Bash 工具
//! 
//! 在 Agent 环境中执行 Bash 命令。

use crate::agent::{Tool, ToolContext, ToolResult};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

// 默认命令超时时间：120 秒
const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Bash 工具参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashToolArgs {
    /// 要执行的命令
    pub command: String,
    /// 命令用途描述（5-10 个词）
    pub description: String,
    /// 超时时间（毫秒），默认 120000
    #[serde(default)]
    pub timeout: Option<u64>,
    /// 工作目录
    #[serde(default)]
    pub workdir: Option<String>,
}

/// Bash 工具
/// 
/// 允许 Agent 执行 Bash 命令。
pub struct BashTool;

impl BashTool {
    /// 创建新的 Bash 工具实例
    pub fn new() -> Self {
        Self
    }

    /// 执行 Bash 命令
    /// 
    /// 返回 (输出, 是否超时, 退出码)
    async fn run_command(
        command: &str,
        workdir: Option<&str>,
        timeout_ms: u64,
    ) -> Result<(String, Option<bool>, i32)> {
        // 构建命令
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // 设置工作目录
        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }

        // 启动进程
        let child = cmd.spawn()?;

        // 设置超时
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

        // 处理结果
        match result {
            Ok(Ok((stdout, stderr, exit_code))) => {
                // 合并 stdout 和 stderr
                let combined = if stderr.is_empty() {
                    stdout
                } else {
                    format!("{}{}", stdout, stderr)
                };
                Ok((combined, None, exit_code))
            }
            Ok(Err(e)) => Err(anyhow!("命令执行错误: {}", e)),
            // 超时
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
                        "description": "要执行的 Bash 命令"
                    },
                    "description": {
                        "type": "string",
                        "description": "命令用途简述（5-10 个词）"
                    },
                    "timeout": {
                        "type": "number",
                        "description": "超时时间（毫秒），默认 120000"
                    },
                    "workdir": {
                        "type": "string",
                        "description": "工作目录"
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
            let args = args.map_err(|e| anyhow!("参数无效: {}", e))?;

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