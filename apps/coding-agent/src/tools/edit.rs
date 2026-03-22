//! 文件编辑工具
//! 
//! 支持多种智能匹配策略的文件内容修改。

use crate::agent::{Tool, ToolContext, ToolResult};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use similar::TextDiff;
use std::path::Path;
use std::pin::Pin;

// ============================================================================
// 参数定义
// ============================================================================

/// 编辑工具参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditToolArgs {
    /// 文件路径
    #[serde(rename = "filePath")]
    pub file_path: String,
    /// 原字符串（要替换的内容）
    #[serde(rename = "oldString")]
    pub old_string: String,
    /// 新字符串（替换后的内容）
    #[serde(rename = "newString")]
    pub new_string: String,
    /// 是否替换所有匹配项
    #[serde(rename = "replaceAll")]
    pub replace_all: Option<bool>,
}

/// 编辑工具
pub struct EditTool;

impl EditTool {
    /// 创建新的编辑工具实例
    pub fn new() -> Self {
        Self
    }
}

// ============================================================================
// 字符串匹配算法
// ============================================================================

/// 计算两个字符串的编辑距离（Levenshtein 距离）
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

// ============================================================================
// 搜索策略
// ============================================================================

/// 策略一：简单精确匹配
fn find_simple(content: &str, old: &str) -> Option<(usize, String)> {
    if content.contains(old) {
        Some((content.find(old).unwrap(), old.to_string()))
    } else {
        None
    }
}

/// 策略二：按行匹配（忽略行首尾空白）
fn find_line_trimmed(content: &str, old: &str) -> Option<(usize, String)> {
    let content_lines: Vec<&str> = content.split('\n').collect();
    let search_lines: Vec<&str> = old.split('\n').collect();

    if search_lines.is_empty() {
        return None;
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

/// 策略三：块匹配（使用首尾行作为锚点，Levenshtein 相似度）
fn find_block_anchor(content: &str, old: &str) -> Option<(usize, String)> {
    let content_lines: Vec<&str> = content.split('\n').collect();
    let search_lines: Vec<&str> = old.split('\n').collect();

    // 至少需要 3 行才能使用块匹配
    if search_lines.len() < 3 {
        return None;
    }

    let first_line_search = search_lines[0].trim();
    let last_line_search = search_lines[search_lines.len() - 1].trim();

    if first_line_search.is_empty() || last_line_search.is_empty() {
        return None;
    }

    // 找到所有可能的候选块（首尾行匹配）
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

    // 计算相似度并找到最佳匹配
    let search_block_size = search_lines.len();
    let mut best_match: Option<(usize, String)> = None;
    let mut max_similarity = -1.0f64;

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

    // 相似度阈值：0.3
    if max_similarity >= 0.3 {
        best_match
    } else {
        None
    }
}

/// 策略四：空白归一化匹配
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

/// 使用多种策略查找匹配
fn find_with_replacers(content: &str, old: &str, _replace_all: bool) -> Result<(usize, String)> {
    // 尝试简单匹配
    if let Some((idx, matched)) = find_simple(content, old) {
        return Ok((idx, matched));
    }

    // 尝试按行匹配
    if let Some((idx, matched)) = find_line_trimmed(content, old) {
        return Ok((idx, matched));
    }

    // 尝试空白归一化匹配
    if let Some((idx, matched)) = find_whitespace_normalized(content, old) {
        return Ok((idx, matched));
    }

    // 尝试块匹配
    if let Some((idx, matched)) = find_block_anchor(content, old) {
        return Ok((idx, matched));
    }

    Err(anyhow!("Could not find oldString in the file"))
}

// ============================================================================
// Tool trait 实现
// ============================================================================

impl Tool for EditTool {
    fn define(&self) -> crate::agent::types::ToolDefine {
        crate::agent::types::ToolDefine {
            name: "edit".to_string(),
            description: "编辑现有文件的内容".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "filePath": {
                        "type": "string",
                        "description": "要修改的文件绝对路径"
                    },
                    "oldString": {
                        "type": "string",
                        "description": "要替换的文本"
                    },
                    "newString": {
                        "type": "string",
                        "description": "替换后的文本"
                    },
                    "replaceAll": {
                        "type": "boolean",
                        "description": "是否替换所有匹配项（默认 false）"
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
            let args = args.map_err(|e| anyhow!("参数无效: {}", e))?;

            // 检查是否有实际修改
            if args.old_string == args.new_string {
                return Err(anyhow!("No changes: oldString and newString are identical"));
            }

            let file_path = Path::new(&args.file_path);

            // 验证文件存在
            if !file_path.exists() {
                return Err(anyhow!("File not found: {}", args.file_path));
            }

            // 确保是文件而非目录
            if file_path.is_dir() {
                return Err(anyhow!("路径是目录而非文件: {}", args.file_path));
            }

            let old_content = tokio::fs::read_to_string(file_path).await?;

            let replace_all = args.replace_all.unwrap_or(false);

            // 使用多种策略查找匹配
            let (idx, matched) = find_with_replacers(&old_content, &args.old_string, replace_all)?;

            // 执行替换
            let new_content = if replace_all {
                old_content.replace(&matched, &args.new_string)
            } else {
                let before = &old_content[..idx];
                let after = &old_content[idx + matched.len()..];
                format!("{}{}{}", before, args.new_string, after)
            };

            // 计算 diff 统计
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

            // 写入文件
            tokio::fs::write(file_path, &new_content).await?;

            Ok(ToolResult::new(
                args.file_path.split('/').last().unwrap_or("file"),
                "编辑成功",
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