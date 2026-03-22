use crate::agent::{Tool, ToolContext, ToolResult};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use similar::TextDiff;
use std::path::Path;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditToolArgs {
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "oldString")]
    pub old_string: String,
    #[serde(rename = "newString")]
    pub new_string: String,
    #[serde(rename = "replaceAll")]
    pub replace_all: Option<bool>,
}

pub struct EditTool;

impl EditTool {
    pub fn new() -> Self {
        Self
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    if a.is_empty() || b.is_empty() {
        return a.len().max(b.len());
    }

    let len_a = a.len();
    let len_b = b.len();

    let mut matrix = vec![vec![0usize; len_b + 1]; len_a + 1];

    for i in 0..=len_a {
        matrix[i][0] = i;
    }
    for j in 0..=len_b {
        matrix[0][j] = j;
    }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a.chars().nth(i - 1) == b.chars().nth(j - 1) { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                matrix[i - 1][j] + 1,
                std::cmp::min(matrix[i][j - 1] + 1, matrix[i - 1][j - 1] + cost),
            );
        }
    }

    matrix[len_a][len_b]
}

fn find_simple(content: &str, old: &str) -> Option<(usize, String)> {
    if content.contains(old) {
        Some((content.find(old).unwrap(), old.to_string()))
    } else {
        None
    }
}

fn find_line_trimmed(content: &str, old: &str) -> Option<(usize, String)> {
    let content_lines: Vec<&str> = content.split('\n').collect();
    let search_lines: Vec<&str> = old.split('\n').collect();

    if search_lines.is_empty() {
        return None;
    }

    let last_search_line = search_lines[search_lines.len() - 1].trim();
    if last_search_line.is_empty() && search_lines.len() > 1 {
        // Handle case where old ends with empty line
    }

    for i in 0..=(content_lines.len().saturating_sub(search_lines.len())) {
        let mut matches = true;
        for j in 0..search_lines.len() {
            let content_line = content_lines[i + j].trim();
            let search_line = search_lines[j].trim();
            if content_line != search_line {
                matches = false;
                break;
            }
        }
        if matches {
            let match_start = content_lines[..i].join("\n").len()
                + if i > 0 { 1 } else { 0 };
            let match_end = content_lines[..i + search_lines.len()].join("\n").len();
            let matched = &content[match_start..match_end];
            return Some((match_start, matched.to_string()));
        }
    }
    None
}

fn find_block_anchor(content: &str, old: &str) -> Option<(usize, String)> {
    let content_lines: Vec<&str> = content.split('\n').collect();
    let search_lines: Vec<&str> = old.split('\n').collect();

    if search_lines.len() < 3 {
        return None;
    }

    let first_line_search = search_lines[0].trim();
    let last_line_search = search_lines[search_lines.len() - 1].trim();

    if first_line_search.is_empty() || last_line_search.is_empty() {
        return None;
    }

    let mut candidates: Vec<(usize, usize)> = Vec::new();

    for i in 0..content_lines.len() {
        if content_lines[i].trim() != first_line_search {
            continue;
        }

        for j in (i + 2)..content_lines.len() {
            if content_lines[j].trim() == last_line_search {
                candidates.push((i, j));
                break;
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    let search_block_size = search_lines.len();
    let mut best_match: Option<(usize, String)> = None;
    let mut max_similarity = -1.0f64;

    if candidates.len() == 1 {
        let (start_line, end_line) = candidates[0];
        let lines_to_check = std::cmp::min(search_block_size - 2, end_line - start_line - 1);

        let similarity = if lines_to_check > 0 {
            let mut total_similarity = 0.0f64;
            for j in 1..=lines_to_check {
                let original_line = content_lines[start_line + j].trim();
                let search_line = search_lines[j].trim();
                let max_len = original_line.len().max(search_line.len());
                if max_len == 0 {
                    continue;
                }
                let distance = levenshtein(original_line, search_line);
                total_similarity += 1.0 - (distance as f64 / max_len as f64);
            }
            total_similarity / lines_to_check as f64
        } else {
            1.0
        };

        if similarity >= 0.0 {
            let match_start = content_lines[..start_line].join("\n").len()
                + if start_line > 0 { 1 } else { 0 };
            let match_end = content_lines[..=end_line].join("\n").len();
            let matched = content[match_start..match_end].to_string();
            return Some((match_start, matched));
        }
        return None;
    }

    for (start_line, end_line) in candidates {
        let lines_to_check = std::cmp::min(search_block_size - 2, end_line - start_line - 1);

        let similarity = if lines_to_check > 0 {
            let mut total_similarity = 0.0f64;
            for j in 1..=lines_to_check {
                let original_line = content_lines[start_line + j].trim();
                let search_line = search_lines[j].trim();
                let max_len = original_line.len().max(search_line.len());
                if max_len == 0 {
                    continue;
                }
                let distance = levenshtein(original_line, search_line);
                total_similarity += 1.0 - (distance as f64 / max_len as f64);
            }
            total_similarity / lines_to_check as f64
        } else {
            1.0
        };

        if similarity > max_similarity {
            max_similarity = similarity;

            let match_start = content_lines[..start_line].join("\n").len()
                + if start_line > 0 { 1 } else { 0 };
            let match_end = content_lines[..=end_line].join("\n").len();
            let matched = content[match_start..match_end].to_string();

            best_match = Some((match_start, matched));
        }
    }

    if max_similarity >= 0.3 {
        best_match
    } else {
        None
    }
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn find_whitespace_normalized(content: &str, old: &str) -> Option<(usize, String)> {
    let normalized_old = normalize_whitespace(old);
    let lines: Vec<&str> = content.split('\n').collect();
    let old_lines: Vec<&str> = old.split('\n').collect();

    if old_lines.len() == 1 {
        for line in &lines {
            if normalize_whitespace(line) == normalized_old {
                return Some((content.find(line).unwrap(), line.to_string()));
            }
        }
        return None;
    }

    for i in 0..=(lines.len().saturating_sub(old_lines.len())) {
        let block: String = lines[i..i + old_lines.len()].join("\n");
        if normalize_whitespace(&block) == normalized_old {
            let match_start = if i == 0 { 0 } else {
                lines[..i].join("\n").len() + 1
            };
            let matched = &content[match_start..match_start + block.len()];
            return Some((match_start, matched.to_string()));
        }
    }

    None
}

fn find_with_replacers(content: &str, old: &str, _replace_all: bool) -> Result<(usize, String)> {
    if let Some((idx, matched)) = find_simple(content, old) {
        return Ok((idx, matched));
    }

    if let Some((idx, matched)) = find_line_trimmed(content, old) {
        return Ok((idx, matched));
    }

    if let Some((idx, matched)) = find_whitespace_normalized(content, old) {
        return Ok((idx, matched));
    }

    if let Some((idx, matched)) = find_block_anchor(content, old) {
        return Ok((idx, matched));
    }

    Err(anyhow!("Could not find oldString in the file."))
}

impl Tool for EditTool {
    fn define(&self) -> crate::agent::types::ToolDefine {
        crate::agent::types::ToolDefine {
            name: "edit".to_string(),
            description: "Make a linear edit to an existing file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "filePath": {
                        "type": "string",
                        "description": "The absolute path to the file to modify"
                    },
                    "oldString": {
                        "type": "string",
                        "description": "The text to replace"
                    },
                    "newString": {
                        "type": "string",
                        "description": "The text to replace it with"
                    },
                    "replaceAll": {
                        "type": "boolean",
                        "description": "Replace all occurrences (default false)"
                    }
                },
                "required": ["filePath", "oldString", "newString"]
            }),
        }
    }

    fn execute(
        &self,
        args: serde_json::Value,
        _ctx: ToolContext,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult>> + Send + '_>> {
        let args: Result<EditToolArgs, _> = serde_json::from_value(args);

        Box::pin(async move {
            let args = args.map_err(|e| anyhow!("Invalid args: {}", e))?;

            if args.old_string == args.new_string {
                return Err(anyhow!("No changes to apply: oldString and newString are identical."));
            }

            let file_path = Path::new(&args.file_path);

            if !file_path.exists() {
                return Err(anyhow!("File not found: {}", args.file_path));
            }

            if file_path.is_dir() {
                return Err(anyhow!("Path is a directory, not a file: {}", args.file_path));
            }

            let old_content = tokio::fs::read_to_string(file_path).await?;

            let replace_all = args.replace_all.unwrap_or(false);

            let (idx, matched) = find_with_replacers(&old_content, &args.old_string, replace_all)?;

            let new_content = if replace_all {
                old_content.replace(&matched, &args.new_string)
            } else {
                let before = &old_content[..idx];
                let after = &old_content[idx + matched.len()..];
                format!("{}{}{}", before, args.new_string, after)
            };

            let diff = TextDiff::from_lines(&old_content, &new_content);
            let mut additions = 0i64;
            let mut deletions = 0i64;
            for change in diff.iter_all_changes() {
                match change.tag() {
                    similar::ChangeTag::Insert => additions += 1,
                    similar::ChangeTag::Delete => deletions += 1,
                    similar::ChangeTag::Equal => {}
                }
            }

            tokio::fs::write(file_path, &new_content).await?;

            Ok(ToolResult::new(
                args.file_path.split('/').last().unwrap_or("file"),
                "Edit applied successfully.",
            ).with_metadata(serde_json::json!({
                "additions": additions,
                "deletions": deletions,
            })))
        })
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}
