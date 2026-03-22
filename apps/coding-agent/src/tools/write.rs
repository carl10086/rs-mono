use crate::agent::{Tool, ToolContext, ToolResult};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use similar::TextDiff;
use std::path::{Path, PathBuf};
use std::pin::Pin;

const DESCRIPTION: &str = include_str!("write.txt");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteToolArgs {
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub content: String,
}

pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self {
        Self
    }

    fn resolve_path(path: &str) -> PathBuf {
        let path = if path.starts_with("~") {
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

    fn extract_title(file_path: &str) -> String {
        Path::new(file_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string())
    }

    fn generate_diff(old_content: &str, new_content: &str, file_path: &str) -> String {
        let diff = TextDiff::from_lines(old_content, new_content);
        diff.unified_diff()
            .header(&format!("a/{}", file_path), &format!("b/{}", file_path))
            .to_string()
    }
}

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
                        "description": "Path to the file to write (relative or absolute)"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
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

            if !parent_dir.exists() {
                tokio::fs::create_dir_all(parent_dir).await.map_err(|e| {
                    anyhow!("Failed to create directory {:?}: {}", parent_dir, e)
                })?;
            }

            let old_content = if file_path.exists() {
                tokio::fs::read_to_string(&file_path).await.unwrap_or_default()
            } else {
                String::new()
            };

            let diff = if old_content.is_empty() {
                String::new()
            } else {
                Self::generate_diff(&old_content, &args.content, &args.file_path)
            };

            // TODO: FileTime verification
            // - Check if file was read in this session before overwriting
            // - Verify file hasn't been modified since last read
            // - Reference: refer/opencode/packages/opencode/src/file/time.ts

            tokio::fs::write(&file_path, &args.content).await.map_err(|e| {
                anyhow!("Failed to write file: {}", e)
            })?;

            // TODO: Event publishing
            // - Publish File.Event.Edited
            // - Publish FileWatcher.Event.Updated
            // - Reference: refer/opencode/packages/opencode/src/tool/write.ts:45-51

            // TODO: LSP integration
            // - Call LSP.touchFile() to trigger re-diagnostics
            // - Get and return LSP diagnostics in output
            // - Reference: refer/opencode/packages/opencode/src/tool/write.ts:55-72

            let title = Self::extract_title(&args.file_path);
            let mut result = ToolResult::new(
                title,
                format!("Successfully wrote {} bytes to {}", args.content.len(), args.file_path),
            );
            
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