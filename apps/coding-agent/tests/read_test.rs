use coding_agent::tools::read::ReadTool;
use coding_agent::agent::{Tool, ToolContext};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tempfile::NamedTempFile;

fn create_test_context() -> ToolContext {
    ToolContext {
        session_id: "test-session".to_string(),
        message_id: "test-message".to_string(),
        agent_name: "test-agent".to_string(),
        abort: Arc::new(AtomicBool::new(false)),
    }
}

fn create_temp_file_with_ext(extension: &str) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let temp_file = NamedTempFile::new_in(&temp_dir).unwrap();
    let path = temp_file.into_temp_path();
    let original_path = path.to_path_buf();
    
    let filename = format!("test{}", extension);
    let new_path = temp_dir.join(&filename);
    
    std::fs::rename(&original_path, &new_path).unwrap_or_else(|_| {
        std::fs::copy(&original_path, &new_path).unwrap();
        std::fs::remove_file(&original_path).ok();
    });
    
    new_path
}

fn cleanup_temp_file(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
}

mod binary_detection {
    use super::*;

    #[tokio::test]
    async fn detects_binary_by_extension_zip() {
        let path = create_temp_file_with_ext(".zip");
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy()
        });
        
        let result = tool.execute(args, create_test_context()).await;
        cleanup_temp_file(&path);
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("binary"), "Should reject binary file by extension");
    }

    #[tokio::test]
    async fn detects_binary_by_extension_exe() {
        let path = create_temp_file_with_ext(".exe");
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy()
        });
        
        let result = tool.execute(args, create_test_context()).await;
        cleanup_temp_file(&path);
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("binary"), "Should reject binary file by extension");
    }

    #[tokio::test]
    async fn detects_binary_by_extension_dll() {
        let path = create_temp_file_with_ext(".dll");
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy()
        });
        
        let result = tool.execute(args, create_test_context()).await;
        cleanup_temp_file(&path);
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("binary"), "Should reject binary file by extension");
    }

    #[tokio::test]
    async fn detects_binary_by_extension_so() {
        let path = create_temp_file_with_ext(".so");
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy()
        });
        
        let result = tool.execute(args, create_test_context()).await;
        cleanup_temp_file(&path);
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("binary"), "Should reject binary file by extension");
    }

    #[tokio::test]
    async fn detects_binary_by_extension_class() {
        let path = create_temp_file_with_ext(".class");
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy()
        });
        
        let result = tool.execute(args, create_test_context()).await;
        cleanup_temp_file(&path);
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("binary"), "Should reject binary file by extension");
    }

    #[tokio::test]
    async fn detects_binary_by_extension_pyc() {
        let path = create_temp_file_with_ext(".pyc");
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy()
        });
        
        let result = tool.execute(args, create_test_context()).await;
        cleanup_temp_file(&path);
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("binary"), "Should reject binary file by extension");
    }
}

mod byte_limit {
    use super::*;

    #[tokio::test]
    async fn truncates_content_exceeding_max_bytes() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_byte_limit.txt");
        
        let content = "a".repeat(60 * 1024);
        std::fs::write(&path, &content).unwrap();
        
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy()
        });
        
        let result = tool.execute(args, create_test_context()).await;
        std::fs::remove_file(&path).ok();
        
        assert!(result.is_ok());
        let tool_result = result.unwrap();
        let output = tool_result.output;
        
        assert!(output.contains("50 KB") || output.contains("51200"), 
            "Should indicate 50KB byte limit");
        assert!(output.contains("offset="), 
            "Should suggest next offset to continue reading");
    }

    #[tokio::test]
    async fn does_not_truncate_content_within_limit() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_small.txt");
        
        let content = "hello world\nline 2\nline 3";
        std::fs::write(&path, &content).unwrap();
        
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy()
        });
        
        let result = tool.execute(args, create_test_context()).await;
        std::fs::remove_file(&path).ok();
        
        assert!(result.is_ok());
        let tool_result = result.unwrap();
        let output = tool_result.output;
        
        assert!(output.contains("End of file"), "Small file should show end of file");
    }
}

mod image_pdf_support {
    use super::*;

    fn create_temp_image_file() -> (PathBuf, &'static str) {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_image.png");
        
        let png_header: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
        ];
        std::fs::write(&path, png_header).unwrap();
        
        (path, "image/png")
    }

    fn create_temp_pdf_file() -> (PathBuf, &'static str) {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test.pdf");
        
        let pdf_header: &[u8] = b"%PDF-1.4";
        std::fs::write(&path, pdf_header).unwrap();
        
        (path, "application/pdf")
    }

    #[tokio::test]
    async fn returns_attachment_for_image() {
        let (path, expected_mime) = create_temp_image_file();
        
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy()
        });
        
        let result = tool.execute(args, create_test_context()).await;
        std::fs::remove_file(&path).ok();
        
        assert!(result.is_ok());
        let tool_result = result.unwrap();
        
        assert!(!tool_result.attachments.is_empty(), "Should have attachments for image");
        assert_eq!(tool_result.attachments[0].mime, expected_mime);
        assert!(!tool_result.attachments[0].data.is_empty(), "Should have base64 data");
    }

    #[tokio::test]
    async fn returns_attachment_for_pdf() {
        let (path, expected_mime) = create_temp_pdf_file();
        
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "filePath": path.to_string_lossy()
        });
        
        let result = tool.execute(args, create_test_context()).await;
        std::fs::remove_file(&path).ok();
        
        assert!(result.is_ok());
        let tool_result = result.unwrap();
        
        assert!(!tool_result.attachments.is_empty(), "Should have attachments for PDF");
        assert_eq!(tool_result.attachments[0].mime, expected_mime);
        assert!(!tool_result.attachments[0].data.is_empty(), "Should have base64 data");
    }
}