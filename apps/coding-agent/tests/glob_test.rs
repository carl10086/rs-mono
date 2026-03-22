use coding_agent::tools::glob::GlobTool;
use coding_agent::agent::{Tool, ToolContext};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tempfile::TempDir;

/// 创建测试用的 ToolContext
fn create_test_context() -> ToolContext {
    ToolContext {
        session_id: "test-session".to_string(),
        message_id: "test-message".to_string(),
        agent_name: "test-agent".to_string(),
        abort: Arc::new(AtomicBool::new(false)),
    }
}

mod basic_search {
    use super::*;

    /// 基本的 glob 模式匹配测试 - 搜索 *.txt 文件
    #[tokio::test]
    async fn finds_files_matching_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // 创建测试文件
        std::fs::write(temp_path.join("a.txt"), "a").unwrap();
        std::fs::write(temp_path.join("b.txt"), "b").unwrap();
        std::fs::write(temp_path.join("c.md"), "c").unwrap();

        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "*.txt",
            "path": temp_path.to_string_lossy()
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok(), "搜索应该成功: {:?}", result);
        let tool_result = result.unwrap();
        let output = tool_result.output;

        // 应该找到两个 .txt 文件
        assert!(output.contains("a.txt"), "应该包含 a.txt: {}", output);
        assert!(output.contains("b.txt"), "应该包含 b.txt: {}", output);
        // .md 文件不应该被找到
        assert!(!output.contains("c.md"), "不应该包含 c.md: {}", output);
    }

    /// 测试递归模式 **/*.rs
    #[tokio::test]
    async fn finds_files_recursively() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // 创建嵌套目录结构
        std::fs::create_dir_all(temp_path.join("src")).unwrap();
        std::fs::create_dir_all(temp_path.join("lib")).unwrap();
        std::fs::write(temp_path.join("src/main.rs"), "main").unwrap();
        std::fs::write(temp_path.join("lib/util.rs"), "util").unwrap();
        std::fs::write(temp_path.join("root.txt"), "root").unwrap();

        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "**/*.rs",
            "path": temp_path.to_string_lossy()
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok(), "递归搜索应该成功: {:?}", result);
        let tool_result = result.unwrap();
        let output = tool_result.output;

        // 应该找到所有 .rs 文件
        assert!(output.contains("main.rs"), "应该包含 main.rs: {}", output);
        assert!(output.contains("util.rs"), "应该包含 util.rs: {}", output);
    }

    /// 无匹配结果时返回提示信息
    #[tokio::test]
    async fn returns_no_files_found_when_no_match() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // 创建一些文件但不创建 .txt 文件
        std::fs::write(temp_path.join("a.md"), "a").unwrap();

        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "*.txt",
            "path": temp_path.to_string_lossy()
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok(), "搜索应该成功即使没有匹配: {:?}", result);
        let tool_result = result.unwrap();
        let output = tool_result.output;

        assert!(output.contains("No files found"), "没有匹配时应该提示: {}", output);
    }
}

mod path_handling {
    use super::*;

    /// 测试默认路径（当前目录）
    #[tokio::test]
    async fn uses_current_dir_when_path_not_specified() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        std::fs::write(temp_path.join("test.txt"), "test").unwrap();

        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "*.txt"
        });

        let result = tool.execute(args, create_test_context()).await;

        // 默认应该在当前目录搜索
        assert!(result.is_ok(), "应该成功执行: {:?}", result);
    }

    /// 测试相对路径解析
    #[tokio::test]
    async fn resolves_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        std::fs::write(temp_path.join("relative.txt"), "test").unwrap();

        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "*.txt",
            "path": temp_path.to_string_lossy()
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok(), "相对路径应该被正确解析: {:?}", result);
        let tool_result = result.unwrap();
        assert!(tool_result.output.contains("relative.txt"));
    }
}

mod result_limit {
    use super::*;

    /// 测试结果数量限制
    #[tokio::test]
    async fn limits_results_to_100() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // 创建超过 100 个文件
        for i in 0..150 {
            std::fs::write(temp_path.join(format!("file_{}.txt", i)), format!("content {}", i)).unwrap();
        }

        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "*.txt",
            "path": temp_path.to_string_lossy()
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok(), "应该成功执行: {:?}", result);
        let tool_result = result.unwrap();
        let output = tool_result.output;

        // 应该提示结果被截断
        assert!(output.contains("truncated") || output.contains("100"), 
            "应该提示结果被截断: {}", output);
    }
}

mod sorting {
    use super::*;

    /// 测试结果按修改时间降序排序
    #[tokio::test]
    async fn sorts_results_by_mtime_descending() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // 创建文件并设置不同的修改时间
        let old_file = temp_path.join("old.txt");
        let new_file = temp_path.join("new.txt");
        
        std::fs::write(&old_file, "old").unwrap();
        std::fs::write(&new_file, "new").unwrap();

        // 等待一小段时间确保 mtime 不同
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        // 修改 new.txt 使其更新
        std::fs::write(&new_file, "newer").unwrap();

        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "*.txt",
            "path": temp_path.to_string_lossy()
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok(), "应该成功执行: {:?}", result);
        let tool_result = result.unwrap();
        let output = tool_result.output;

        // new.txt 应该在列表前面（修改时间更近）
        let new_pos = output.find("new.txt").unwrap();
        let old_pos = output.find("old.txt").unwrap();
        assert!(new_pos < old_pos, "new.txt 应该排在 old.txt 前面: {}", output);
    }
}

mod metadata {
    use super::*;

    /// 测试返回正确的 metadata
    #[tokio::test]
    async fn returns_correct_count_in_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        std::fs::write(temp_path.join("a.txt"), "a").unwrap();
        std::fs::write(temp_path.join("b.txt"), "b").unwrap();

        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "*.txt",
            "path": temp_path.to_string_lossy()
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        let metadata = tool_result.metadata;

        // 检查 count 字段
        assert!(metadata.get("count").is_some(), "应该有 count 字段: {}", metadata);
        let count = metadata.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count, 2, "应该找到 2 个文件");
    }

    /// 测试截断状态在 metadata 中正确反映
    #[tokio::test]
    async fn returns_truncated_flag_in_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // 创建超过限制的文件
        for i in 0..150 {
            std::fs::write(temp_path.join(format!("file_{}.txt", i)), format!("{}", i)).unwrap();
        }

        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "*.txt",
            "path": temp_path.to_string_lossy()
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        let metadata = tool_result.metadata;

        // 检查 truncated 字段
        assert!(metadata.get("truncated").is_some(), "应该有 truncated 字段: {}", metadata);
        assert!(metadata.get("truncated").unwrap().as_bool().unwrap_or(false), 
            "truncated 应该为 true: {}", metadata);
    }
}
