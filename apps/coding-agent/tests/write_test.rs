use coding_agent::tools::write::WriteTool;
use coding_agent::agent::{Tool, ToolContext};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::path::PathBuf;
use std::fs;

fn create_test_context() -> ToolContext {
    ToolContext {
        session_id: "test-session".to_string(),
        message_id: "test-message".to_string(),
        agent_name: "test-agent".to_string(),
        abort: Arc::new(AtomicBool::new(false)),
    }
}

fn cleanup_temp_file(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

fn cleanup_temp_dir(path: &PathBuf) {
    let _ = fs::remove_dir_all(path);
}

mod basic_write {
    use super::*;

    #[tokio::test]
    async fn writes_new_file_successfully() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_write_{}.txt", uuid::Uuid::new_v4()));
        let path_clone = path.clone();

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "content": "hello world"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok(), "Write should succeed: {:?}", result.err());
        assert!(path.exists(), "File should exist after write");
        cleanup_temp_file(&path_clone);
    }

    #[tokio::test]
    async fn writes_correct_content() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_write_{}.txt", uuid::Uuid::new_v4()));
        let path_clone = path.clone();
        let content = "hello rust";

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "content": content
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok());
        let read_content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path_clone);
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn overwrites_existing_file() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_write_{}.txt", uuid::Uuid::new_v4()));
        let path_clone = path.clone();

        fs::write(&path, "original content").unwrap();

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "content": "new content"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok());
        let read_content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path_clone);
        assert_eq!(read_content, "new content");
    }

    #[tokio::test]
    async fn creates_parent_directories() {
        let temp_dir = std::env::temp_dir();
        let nested_dir = temp_dir.join(format!("test_write_nested_{}", uuid::Uuid::new_v4()));
        let path = nested_dir.join("subdir").join("file.txt");
        let path_clone = path.clone();
        let nested_dir_clone = nested_dir.clone();

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "content": "nested content"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok(), "Should create parent dirs: {:?}", result.err());
        assert!(path.exists(), "File should exist");
        let read_content = fs::read_to_string(&path).unwrap();
        cleanup_temp_dir(&nested_dir_clone);
        assert_eq!(read_content, "nested content");
    }
}

mod diff_generation {
    use super::*;

    #[tokio::test]
    async fn generates_diff_when_overwriting() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_write_diff_{}.txt", uuid::Uuid::new_v4()));
        let path_clone = path.clone();

        fs::write(&path, "line 1\nline 2\nline 3").unwrap();

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "content": "line 1\nmodified line 2\nline 3"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok(), "Write should succeed: {:?}", result.err());
        let result = result.unwrap();
        
        assert!(result.metadata.get("diff").is_some(), "Should have diff in metadata");
        let diff = result.metadata.get("diff").unwrap().as_str().unwrap();
        assert!(diff.contains("-line 2"), "Diff should show removed line");
        assert!(diff.contains("+modified line 2"), "Diff should show added line");
        
        cleanup_temp_file(&path_clone);
    }

    #[tokio::test]
    async fn returns_empty_diff_for_new_file() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_write_diff_new_{}.txt", uuid::Uuid::new_v4()));
        let path_clone = path.clone();

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "content": "brand new content"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        
        let diff = result.metadata.get("diff").and_then(|d| d.as_str());
        assert!(diff.is_none() || diff.unwrap().is_empty(), "New file should have no diff");
        
        cleanup_temp_file(&path_clone);
    }

    #[tokio::test]
    async fn generates_unified_diff_format() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_write_diff_fmt_{}.txt", uuid::Uuid::new_v4()));
        let path_clone = path.clone();

        fs::write(&path, "fn hello() {\n    let x = 1;\n}").unwrap();

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "content": "fn hello() {\n    let x = 100;\n}"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        
        let diff = result.metadata.get("diff").and_then(|d| d.as_str()).unwrap_or("");
        assert!(diff.contains("---"), "Diff should have --- header");
        assert!(diff.contains("+++"), "Diff should have +++ header");
        
        cleanup_temp_file(&path_clone);
    }
}