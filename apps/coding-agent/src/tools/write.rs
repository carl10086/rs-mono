//! 文件写入工具
//! 
//! 支持创建新文件或覆盖已有文件，自动创建父目录。

use crate::agent::{Tool, ToolContext, ToolResult};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use similar::TextDiff;
use std::path::{Path, PathBuf};
use std::pin::Pin;

const DESCRIPTION: &str = include_str!("write.txt");

// ============================================================================
// 工具参数
// ============================================================================

/// 写入工具参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteToolArgs {
    /// 文件路径（绝对或相对路径）
    #[serde(rename = "filePath")]
    pub file_path: String,
    /// 文件内容
    pub content: String,
}

/// 写入工具
pub struct WriteTool;

impl WriteTool {
    /// 创建新的写入工具实例
    pub fn new() -> Self {
        Self
    }

    /// 解析路径，支持 ~ 展开和相对路径转换
    fn resolve_path(path: &str) -> PathBuf {
        let path = if path.starts_with('~') {
            dirs::home_dir()
                .map(|home| path.replacen("~", &home.to_string_lossy(), 1))
                .unwrap_or_else(|| path.to_string())
        } else {
            path.to_string()
        };

        if Path::new(&path).is_absolute() {
            PathBuf::from(&path)
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(&path))
                .unwrap_or_else(|_| PathBuf::from(&path))
        }
    }

    /// 从路径中提取文件名作为标题
    fn extract_title(file_path: &str) -> String {
        Path::new(file_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string())
    }

    /// 生成 Unified Diff 格式的差异
    fn generate_diff(old_content: &str, new_content: &str, file_path: &str) -> String {
        let diff = TextDiff::from_lines(old_content, new_content);
        diff.unified_diff()
            .header(&format!("a/{}", file_path), &format!("b/{}", file_path))
            .to_string()
    }
}

// ============================================================================
// Tool trait 实现
// ============================================================================

impl Tool for WriteTool {
    fn define(&self) -> crate::agent::types::ToolDefine {
        crate::agent::types::ToolDefine {
            name: "write".to_string(),
            description: DESCRIPTION.to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "filePath": {
                        "type": "string",
                        "description": "要写入的文件路径（绝对或相对路径）"
                    },
                    "content": {
                        "type": "string",
                        "description": "要写入的内容"
                    }
                },
                "required": ["filePath", "content"]
            }),
        }
    }

    fn execute(
        &self,
        args: serde_json::Value,
        _ctx: ToolContext,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult>> + Send + '_>> {
        let args: Result<WriteToolArgs, _> = serde_json::from_value(args);

        Box::pin(async move {
            let args = args.map_err(|e| anyhow!("Invalid args: {}", e))?;

            let file_path = Self::resolve_path(&args.file_path);
            let parent_dir = file_path.parent().ok_or_else(|| {
                anyhow!("Invalid path: {}", args.file_path)
            })?;

            // 自动创建父目录
            if !parent_dir.exists() {
                tokio::fs::create_dir_all(parent_dir).await.map_err(|e| {
                    anyhow!("Failed to create directory {:?}: {}", parent_dir, e)
                })?;
            }

            // 读取旧内容用于生成 diff
            let old_content = if file_path.exists() {
                tokio::fs::read_to_string(&file_path).await.unwrap_or_default()
            } else {
                String::new()
            };

            // 生成差异（如果有旧内容）
            let diff = if old_content.is_empty() {
                String::new()
            } else {
                Self::generate_diff(&old_content, &args.content, &args.file_path)
            };

            // TODO: 文件时间戳验证
            // - 检查文件是否在本次会话中读过
            // - 验证文件自上次读取后是否被修改

            // 写入文件
            tokio::fs::write(&file_path, &args.content).await.map_err(|e| {
                anyhow!("Failed to write file: {}", e)
            })?;

            // TODO: 事件发布
            // - 发布 File.Event.Edited
            // - 发布 FileWatcher.Event.Updated

            // TODO: LSP 集成
            // - 调用 LSP.touchFile() 触发重新诊断
            // - 在输出中返回 LSP 诊断信息

            let title = Self::extract_title(&args.file_path);
            let mut result = ToolResult::new(
                title,
                format!("Successfully wrote {} bytes to {}", args.content.len(), args.file_path),
            );
            
            // 如果有 diff，添加到元数据
            if !diff.is_empty() {
                result = result.with_metadata(serde_json::json!({
                    "diff": diff
                }));
            }

            Ok(result)
        })
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}