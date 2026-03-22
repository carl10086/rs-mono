use coding_agent::tools::edit::EditTool;
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

fn create_temp_file(content: &str) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join(format!("test_edit_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&path, content).unwrap();
    path
}

fn cleanup_temp_file(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

mod basic_edit {
    use super::*;

    #[tokio::test]
    async fn applies_simple_edit() {
        let path = create_temp_file("hello world");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "world",
            "newString": "rust"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Edit should succeed: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("hello rust"), "Content should contain replacement");
    }

    #[tokio::test]
    async fn rejects_identical_old_and_new() {
        let path = create_temp_file("hello world");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "world",
            "newString": "world"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        cleanup_temp_file(&path);
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("identical"), "Should reject identical strings");
    }

    #[tokio::test]
    async fn returns_error_for_nonexistent_file() {
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": "/nonexistent/path/file.txt",
            "oldString": "old",
            "newString": "new"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"), "Should report file not found");
    }

    #[tokio::test]
    async fn returns_error_when_old_string_not_found() {
        let path = create_temp_file("hello world");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "nonexistent",
            "newString": "new"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        cleanup_temp_file(&path);
        assert!(err.to_string().contains("Could not find"), "Should report oldString not found");
    }

    #[tokio::test]
    async fn replaces_only_first_match_by_default() {
        let path = create_temp_file("foo bar foo baz");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "foo",
            "newString": "replaced"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert_eq!(content, "replaced bar foo baz", "Should only replace first match");
    }

    #[tokio::test]
    async fn replaces_all_matches_when_replace_all_true() {
        let path = create_temp_file("foo bar foo baz");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "foo",
            "newString": "replaced",
            "replaceAll": true
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert_eq!(content, "replaced bar replaced baz", "Should replace all matches");
    }
}

mod line_trimmed_replacer {
    use super::*;

    #[tokio::test]
    async fn matches_with_trailing_whitespace() {
        let path = create_temp_file("hello world   \nnext line");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "hello world",
            "newString": "hello rust"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match even with trailing whitespace: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("hello rust"), "Should replace content");
    }

    #[tokio::test]
    async fn matches_with_leading_whitespace() {
        let path = create_temp_file("hello\n   world\nnext");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "world",
            "newString": "rust"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match with leading whitespace: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("rust"), "Should replace content");
    }
}

mod block_anchor_replacer {
    use super::*;

    #[tokio::test]
    async fn matches_block_by_anchors() {
        let path = create_temp_file("fn hello() {\n    let x = 1;\n    let y = 2;\n}\nfn world() {}");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "fn hello() {\n    let x = 1;\n    let y = 2;\n}",
            "newString": "fn hello() {\n    let x = 100;\n    let y = 200;\n}"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match block by anchors: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("let x = 100"), "Should replace middle content");
        assert!(content.contains("fn world() {}"), "Should preserve following content");
    }

    #[tokio::test]
    async fn matches_with_similar_middle_content() {
        let path = create_temp_file("fn hello() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n}\nfn other() {}");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "fn hello() {\n    let a = 1;\n    let b = 2;\n    let c = 999;\n}",
            "newString": "fn hello() {\n    let a = 1;\n    let b = 2;\n    let c = 100;\n}"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match using Levenshtein similarity: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("let c = 100"), "Should update matched block");
    }
}

mod whitespace_normalized_replacer {
    use super::*;

    #[tokio::test]
    async fn matches_with_variable_whitespace() {
        let path = create_temp_file("hello    world\nnext line");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "hello world",
            "newString": "hello rust"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match with variable whitespace: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("hello rust"), "Should replace");
    }

    #[tokio::test]
    async fn matches_multiline_block_with_whitespace_variation() {
        let path = create_temp_file("fn hello() {\n    let  x  =  1;\n    let   y   =   2;\n}");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "fn hello() {\n    let x = 1;\n    let y = 2;\n}",
            "newString": "fn hello() {\n    let x = 100;\n    let y = 200;\n}"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match block ignoring internal whitespace: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("let x = 100"), "Should update x");
        assert!(content.contains("let y = 200"), "Should update y");
    }
}

mod indentation_flexible_replacer {
    use super::*;

    #[tokio::test]
    async fn matches_block_with_different_indentation() {
        let path = create_temp_file("    fn hello() {\n        let x = 1;\n    }");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "fn hello() {\n    let x = 1;\n}",
            "newString": "fn hello() {\n    let x = 100;\n}"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match block with different indentation: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("let x = 100"), "Should update content");
    }

    #[tokio::test]
    async fn preserves_surrounding_indentation() {
        let path = create_temp_file("        fn hello() {\n            let x = 1;\n            let y = 2;\n        }");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "fn hello() {\n    let x = 1;\n    let y = 2;\n}",
            "newString": "fn hello() {\n    let x = 100;\n    let y = 200;\n}"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match with different base indentation: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("let x = 100"), "Should update x");
        assert!(content.contains("let y = 200"), "Should update y");
    }
}

mod line_ending {
    use super::*;

    #[tokio::test]
    async fn preserves_crlf_line_endings() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_edit_crlf_{}.txt", uuid::Uuid::new_v4()));
        fs::write(&path, "line1\r\nline2\r\nline3").unwrap();
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "line2",
            "newString": "modified"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        let content = fs::read_to_string(&path).unwrap();
        fs::remove_file(&path).ok();
        
        assert!(result.is_ok());
        assert!(content.contains("\r\n"), "CRLF should be preserved");
    }

    #[tokio::test]
    async fn preserves_lf_line_endings() {
        let path = create_temp_file("line1\nline2\nline3");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "line2",
            "newString": "modified"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        
        assert!(result.is_ok());
        assert!(content.contains('\n') && !content.contains("\r\n"), "LF should be preserved");
    }
}

mod escape_normalized_replacer {
    use super::*;

    #[tokio::test]
    async fn matches_with_escaped_newline() {
        let path = create_temp_file("prefix hello\\nworld suffix");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "prefix hello\\nworld suffix",
            "newString": "prefix hello\nworld suffix"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match escaped newline: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("hello\nworld"), "Should replace with actual newline");
    }

    #[tokio::test]
    async fn matches_escaped_tab() {
        let path = create_temp_file("prefix hello\\tworld suffix");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "prefix hello\\tworld suffix",
            "newString": "prefix hello\tworld suffix"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match escaped tab: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("hello\tworld"), "Should replace with actual tab");
    }
}

mod trimmed_boundary_replacer {
    use super::*;

    #[tokio::test]
    async fn matches_trimmed_content() {
        let path = create_temp_file("prefix   hello world   suffix");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "  hello world  ",
            "newString": "hello rust"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match trimmed content: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("hello rust"), "Should replace trimmed content");
    }

    #[tokio::test]
    async fn matches_multiline_trimmed_block() {
        let path = create_temp_file("prefix\n  hello\n  world\nsuffix");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "\n  hello\n  world\n",
            "newString": "\nhello rust\n"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match multiline trimmed block: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("hello rust"), "Should replace trimmed block");
    }
}

mod context_aware_replacer {
    use super::*;

    #[tokio::test]
    async fn matches_using_context_anchors() {
        let path = create_temp_file("fn hello() {\n    let x = 1;\n    let y = 2;\n    let z = 3;\n}\nfn other() {}");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "fn hello() {\n    let x = 1;\n    let y = 999;\n}",
            "newString": "fn hello() {\n    let x = 100;\n    let y = 200;\n}"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Should match using context anchors: {:?}", result.err());
        let content = fs::read_to_string(&path).unwrap();
        cleanup_temp_file(&path);
        assert!(content.contains("let x = 100"), "Should update x");
        assert!(content.contains("let y = 200"), "Should update y");
    }
}

mod diff_stats {
    use super::*;

    #[tokio::test]
    async fn returns_additions_and_deletions_in_metadata() {
        let path = create_temp_file("prefix\nline2\nsuffix\n");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "prefix\nline2\nsuffix",
            "newString": "prefix\nline2_modified\nline2_extra\nsuffix"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Edit should succeed");
        let tool_result = result.unwrap();
        cleanup_temp_file(&path);
        
        let metadata = tool_result.metadata;
        assert!(metadata.get("additions").is_some(), "Should have additions in metadata");
        assert!(metadata.get("deletions").is_some(), "Should have deletions in metadata");
        
        let additions = metadata.get("additions").unwrap().as_i64().unwrap();
        let deletions = metadata.get("deletions").unwrap().as_i64().unwrap();
        
        assert_eq!(additions, 2, "Should have 2 additions");
        assert_eq!(deletions, 1, "Should have 1 deletion");
    }

    #[tokio::test]
    async fn counts_multiline_changes() {
        let path = create_temp_file("start\nfn hello() {\n    let x = 1;\n    let y = 2;\n}\nend\n");
        
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy(),
            "oldString": "fn hello() {\n    let x = 1;\n    let y = 2;\n}",
            "newString": "fn hello() {\n    let x = 100;\n    let y = 200;\n    let z = 300;\n}"
        });
        
        let result = tool.execute(args, create_test_context()).await;
        
        assert!(result.is_ok(), "Edit should succeed");
        let tool_result = result.unwrap();
        cleanup_temp_file(&path);
        
        let metadata = tool_result.metadata;
        let additions = metadata.get("additions").unwrap().as_i64().unwrap();
        let deletions = metadata.get("deletions").unwrap().as_i64().unwrap();
        
        assert!(additions > 0, "Should have additions");
        assert_eq!(deletions, 2, "Should have 2 deletions (original 2 lines)");
    }
}
