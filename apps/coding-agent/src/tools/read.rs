use crate::agent::{Tool, ToolContext, ToolResult};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::pin::Pin;

const DESCRIPTION: &str = include_str!("read.txt");

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

const DEFAULT_READ_LIMIT: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;
const MAX_LINE_SUFFIX: &str = "... (line truncated)";
const MAX_BYTES: usize = 50 * 1024;
const MAX_BYTES_LABEL: &str = "50 KB";

const BINARY_EXTENSIONS: &[&str] = &[
    ".zip", ".tar", ".gz", ".exe", ".dll", ".so", ".class", ".jar", ".war",
    ".7z", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx", ".odt", ".ods",
    ".odp", ".bin", ".dat", ".obj", ".o", ".a", ".lib", ".wasm", ".pyc", ".pyo",
];

const IMAGE_MIME_TYPES: &[(&str, &str)] = &[
    (".png", "image/png"),
    (".jpg", "image/jpeg"),
    (".jpeg", "image/jpeg"),
    (".gif", "image/gif"),
    (".webp", "image/webp"),
];

fn get_mime_type(path: &Path) -> Option<&'static str> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())?;
    
    for (extension, mime) in IMAGE_MIME_TYPES {
        if ext == extension[1..] {
            return Some(mime);
        }
    }
    
    if ext == "pdf" {
        return Some("application/pdf");
    }
    
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadToolArgs {
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ReadTool {
    fn define(&self) -> crate::agent::types::ToolDefine {
        crate::agent::types::ToolDefine {
            name: "read".to_string(),
            description: DESCRIPTION.to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "filePath": {
                        "type": "string",
                        "description": "The absolute path to the file or directory to read"
                    },
                    "offset": {
                        "type": "number",
                        "description": "The line number to start reading from (1-indexed)"
                    },
                    "limit": {
                        "type": "number",
                        "description": "The maximum number of lines to read (defaults to 2000)"
                    }
                },
                "required": ["filePath"]
            }),
        }
    }

    fn execute(
        &self,
        args: serde_json::Value,
        _ctx: ToolContext,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult>> + Send + '_>> {
        let args: Result<ReadToolArgs, _> = serde_json::from_value(args);
        let file_path = match args {
            Ok(ref a) => Path::new(&a.file_path).to_path_buf(),
            Err(_) => Path::new("").to_path_buf(),
        };

        Box::pin(async move {
            let args = args.map_err(|e| anyhow!("Invalid args: {}", e))?;

            let metadata = tokio::fs::metadata(&file_path).await?;

            if metadata.is_dir() {
                return Self::read_directory(&args);
            }

            Self::read_file(&args).await
        })
    }
}

impl ReadTool {
    fn is_binary_by_extension(file_path: &Path) -> bool {
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .map(|e| format!(".{}", e));
        
        match ext {
            Some(ref ext) => BINARY_EXTENSIONS.contains(&ext.as_str()),
            None => false,
        }
    }

    async fn read_file(args: &ReadToolArgs) -> anyhow::Result<ToolResult> {
        let file_path = Path::new(&args.file_path);

        if !file_path.exists() {
            return Err(anyhow!("File not found: {}", args.file_path));
        }

        if let Some(mime) = get_mime_type(file_path) {
            let bytes = tokio::fs::read(file_path).await?;
            let base64_data = base64_encode(&bytes);
            let msg = format!("Read {} file", mime.split('/').next().unwrap_or("file"));
            
            return Ok(ToolResult::new(
                args.file_path.split('/').last().unwrap_or("file"),
                msg.clone(),
            )
            .with_metadata(serde_json::json!({
                "preview": msg,
                "truncated": false,
            }))
            .with_attachment(mime, base64_data));
        }

        if Self::is_binary_by_extension(file_path) {
            return Err(anyhow!("Cannot read binary file: {}", args.file_path));
        }

        let file = tokio::fs::File::open(file_path).await?;
        use tokio::io::AsyncReadExt;
        let mut reader = tokio::io::BufReader::new(file);
        
        let mut content = Vec::new();
        let mut total_bytes_read = 0;
        let mut buffer = vec![0u8; 8192];
        
        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            total_bytes_read += bytes_read;
            if total_bytes_read > MAX_BYTES {
                content.extend_from_slice(&buffer[..bytes_read]);
                break;
            }
            content.extend_from_slice(&buffer[..bytes_read]);
        }
        
        let content_str = String::from_utf8_lossy(&content);
        let total_lines = content_str.lines().count();
        
        let limit = args.limit.unwrap_or(DEFAULT_READ_LIMIT);
        let offset = args.offset.unwrap_or(1).saturating_sub(1);

        let lines: Vec<String> = content_str
            .lines()
            .skip(offset)
            .take(limit)
            .enumerate()
            .map(|(i, line)| {
                let line_num = offset + i + 1;
                let truncated = if line.len() > MAX_LINE_LENGTH {
                    format!("{}{}", &line[..MAX_LINE_LENGTH], MAX_LINE_SUFFIX)
                } else {
                    line.to_string()
                };
                format!("{}: {}", line_num, truncated)
            })
            .collect();

        let truncated_by_bytes = total_bytes_read > MAX_BYTES;
        let has_more = offset + limit < total_lines || truncated_by_bytes;
        let last_read_line = offset + lines.len();
        let next_offset = last_read_line + 1;
        
        let mut output = format!(
            "<path>{}</path>\n<type>file</type>\n<content>\n{}\n",
            args.file_path,
            lines.join("\n")
        );
        
        if truncated_by_bytes {
            output.push_str(&format!(
                "\n\n(Output capped at {}. Showing lines {}-{}. Use offset={} to continue.)",
                MAX_BYTES_LABEL,
                offset + 1,
                last_read_line,
                next_offset
            ));
        } else if has_more {
            output.push_str(&format!(
                "\n\n(Showing lines {}-{} of {}. Use offset={} to continue.)",
                offset + 1,
                last_read_line,
                total_lines,
                next_offset
            ));
        } else {
            output.push_str(&format!("\n\n(End of file - total {} lines)", total_lines));
        }
        output.push_str("\n</content>");

        let preview = lines.iter().take(10).cloned().collect::<Vec<_>>().join("\n");

        Ok(ToolResult::new(
            format!(
                "{}.rs",
                args.file_path.split('/').last().unwrap_or("file")
            ),
            output,
        ).with_metadata(serde_json::json!({
            "preview": preview,
            "truncated": has_more,
            "total_lines": total_lines,
        })))
    }

    fn read_directory(args: &ReadToolArgs) -> anyhow::Result<ToolResult> {
        let file_path = Path::new(&args.file_path);

        if !file_path.exists() {
            return Err(anyhow!("Directory not found: {}", args.file_path));
        }

        let mut entries: Vec<String> = std::fs::read_dir(file_path)?
            .filter_map(|e| e.ok())
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if e.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    name + "/"
                } else {
                    name
                }
            })
            .collect();

        entries.sort();

        let limit = args.limit.unwrap_or(DEFAULT_READ_LIMIT);
        let offset = args.offset.unwrap_or(1).saturating_sub(1);
        let sliced: Vec<String> = entries.iter().skip(offset).take(limit).cloned().collect();
        let has_more = offset + limit < entries.len();

        let output = format!(
            "<path>{}</path>\n<type>directory</type>\n<entries>\n{}\n</entries>",
            args.file_path,
            sliced.join("\n")
        );

        let preview = sliced.iter().take(10).cloned().collect::<Vec<_>>().join("\n");

        Ok(ToolResult::new(
            args.file_path.split('/').last().unwrap_or("directory").to_string(),
            output,
        ).with_metadata(serde_json::json!({
            "preview": preview,
            "truncated": has_more,
            "total_entries": entries.len(),
        })))
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}
