use coding_agent::tools::bash::BashTool;
use coding_agent::agent::{Tool, ToolContext};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

fn create_test_context() -> ToolContext {
    ToolContext {
        session_id: "test-session".to_string(),
        message_id: "test-message".to_string(),
        agent_name: "test-agent".to_string(),
        abort: Arc::new(AtomicBool::new(false)),
    }
}

mod basic_execution {
    use super::*;

    #[tokio::test]
    async fn executes_simple_command() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "echo hello",
            "description": "echo hello"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok(), "Command should succeed: {:?}", result.err());
        let output = result.unwrap().output;
        assert!(output.contains("hello"), "Output should contain 'hello': {}", output);
    }

    #[tokio::test]
    async fn returns_exit_code() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "true",
            "description": "true command"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        let exit_code = tool_result.metadata.get("exit_code");
        assert!(exit_code.is_some(), "Should have exit_code in metadata");
    }

    #[tokio::test]
    async fn captures_stderr() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "echo error >&2",
            "description": "echo to stderr"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok());
        let output = result.unwrap().output;
        assert!(output.contains("error"), "Should capture stderr: {}", output);
    }

    #[tokio::test]
    async fn respects_timeout() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "sleep 5",
            "description": "sleep 5 seconds",
            "timeout": 100
        });

        let result = tool.execute(args, create_test_context()).await;

        // Should timeout and return error or set timed_out metadata
        assert!(result.is_ok()); // Command succeeds (no error), but may be timed out
        let tool_result = result.unwrap();
        let timed_out = tool_result.metadata.get("timed_out");
        assert!(timed_out.is_some(), "Should indicate timeout in metadata");
    }

    #[tokio::test]
    async fn respects_workdir() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "pwd",
            "description": "print working directory",
            "workdir": "/tmp"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok());
        let output = result.unwrap().output;
        assert!(output.contains("/tmp"), "Should execute in workdir: {}", output);
    }
}

mod command_validation {
    use super::*;

    #[tokio::test]
    async fn requires_command_parameter() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "description": "missing command"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_err(), "Should fail without command");
    }

    #[tokio::test]
    async fn requires_description_parameter() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "echo test"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_err(), "Should fail without description");
    }

    #[tokio::test]
    async fn handles_command_failure() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "exit 1",
            "description": "exit with code 1"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok()); // Execution succeeds, but exit code is captured
        let tool_result = result.unwrap();
        let exit_code = tool_result.metadata.get("exit_code").and_then(|v| v.as_i64());
        assert_eq!(exit_code, Some(1), "Should capture non-zero exit code");
    }
}

mod output_handling {
    use super::*;

    #[tokio::test]
    async fn handles_multiline_output() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "printf 'line1\\nline2\\nline3'",
            "description": "multiline output"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok());
        let output = result.unwrap().output;
        assert!(output.contains("line1"), "Should contain line1: {}", output);
        assert!(output.contains("line2"), "Should contain line2: {}", output);
        assert!(output.contains("line3"), "Should contain line3: {}", output);
    }

    #[tokio::test]
    async fn handles_empty_output() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "",
            "description": "empty command"
        });

        let result = tool.execute(args, create_test_context()).await;

        assert!(result.is_ok());
        let output = result.unwrap().output;
        assert!(output.is_empty(), "Should handle empty output: '{}'", output);
    }
}