use crate::agent::{Tool, ToolContext, ToolResult};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::pin::Pin;

const DESCRIPTION: &str = include_str!("glob.txt");

/// Glob 工具参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobToolArgs {
    /// Glob 模式，如 "*.rs", "**/*.json"
    pub pattern: String,
    /// 搜索目录路径，默认当前目录
    pub path: Option<String>,
}

/// Glob 工具 - 使用 glob crate 进行文件模式匹配
pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GlobTool {
    fn define(&self) -> crate::agent::types::ToolDefine {
        crate::agent::types::ToolDefine {
            name: "glob".to_string(),
            description: DESCRIPTION.to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The glob pattern to match files against, e.g. '*.rs', '**/*.json', 'src/**/*.ts'"
                    },
                    "path": {
                        "type": "string",
                        "description": "The directory to search in. If not specified, the current working directory will be used."
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    fn execute(
        &self,
        args: serde_json::Value,
        _ctx: ToolContext,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult>> + Send + '_>> {
        let args: Result<GlobToolArgs, _> = serde_json::from_value(args);

        Box::pin(async move {
            let args = args.map_err(|e| anyhow!("Invalid args: {}", e))?;

            // 确定搜索目录
            let search_dir = if let Some(ref path) = args.path {
                if Path::new(path).is_absolute() {
                    path.clone()
                } else {
                    // 相对路径：相对于当前工作目录
                    std::env::current_dir()
                        .map(|cwd| cwd.join(path))
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| path.clone())
                }
            } else {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            };

            // 构建完整的 glob 模式
            let full_pattern = if args.pattern.starts_with('/') {
                args.pattern.clone()
            } else {
                format!("{}/{}", search_dir, args.pattern)
            };

            // 搜索文件
            let mut files: Vec<(String, u64)> = Vec::new();
            let limit = 100;

            for entry in glob::glob(&full_pattern).map_err(|e| anyhow!("Glob pattern error: {}", e))? {
                match entry {
                    Ok(path) => {
                        if path.is_file() {
                            // 获取修改时间
                            let mtime = std::fs::metadata(&path)
                                .and_then(|m| m.modified())
                                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
                                .unwrap_or(0);
                            
                            let path_str = path.to_string_lossy().to_string();
                            files.push((path_str, mtime));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Glob entry error: {:?}", e);
                    }
                }
            }

            // 按修改时间降序排序
            files.sort_by(|a, b| b.1.cmp(&a.1));

            // 检查是否截断
            let truncated = files.len() > limit;
            if truncated {
                files.truncate(limit);
            }

            // 构建输出
            let output = if files.is_empty() {
                "No files found".to_string()
            } else {
                let paths: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
                let mut output = paths.join("\n");
                if truncated {
                    output.push_str(&format!("\n\n(Results are truncated: showing first {} results. Consider using a more specific path or pattern.)", limit));
                }
                output
            };

            // 计算相对路径作为 title
            let title = Path::new(&search_dir)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| search_dir.clone());

            Ok(ToolResult::new(title, output).with_metadata(serde_json::json!({
                "count": files.len(),
                "truncated": truncated,
            })))
        })
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}
