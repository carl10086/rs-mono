use coding_agent::agent::executor::{Tool, ToolExecutor};
use coding_agent::agent::types::{ToolCall, ToolContext, ToolDefine, ToolResult};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// 测试上下文构建器
/// 创建一个用于测试的 ToolContext，避免重复代码
fn new_context() -> ToolContext {
    ToolContext {
        session_id: "test-session".to_string(),
        message_id: "test-message".to_string(),
        agent_name: "test-agent".to_string(),
        abort: Arc::new(AtomicBool::new(false)),
    }
}

/// 一个简单的测试工具，用于验证 ToolExecutor 的注册和执行功能
struct MockTool {
    name: String,
    description: String,
    should_fail: bool,
}

impl MockTool {
    fn success(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: format!("Mock tool: {}", name),
            should_fail: false,
        }
    }

    fn failing(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: format!("Failing mock tool: {}", name),
            should_fail: true,
        }
    }
}

impl Tool for MockTool {
    fn define(&self) -> ToolDefine {
        ToolDefine {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: serde_json::json!({}),
        }
    }

    fn execute(
        &self,
        _args: serde_json::Value,
        _ctx: ToolContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<ToolResult>> + Send + '_>>
    {
        let should_fail = self.should_fail;
        Box::pin(async move {
            if should_fail {
                anyhow::bail!("Mock tool failed")
            }
            Ok(ToolResult::new("success", "Mock tool executed"))
        })
    }
}

// ============================================================================
// 工具执行器核心功能测试
// ============================================================================

mod executor_creation {
    use super::*;

    #[test]
    fn 创建空执行器() {
        let executor = ToolExecutor::new();
        let tools = executor.tools();
        assert!(tools.is_empty(), "新执行器应该没有工具");
    }
}

mod tool_registration {
    use super::*;

    #[test]
    fn 注册单个工具() {
        let executor = ToolExecutor::new();
        let tool = MockTool::success("test-tool");
        let executor = executor.register(tool);

        let tools = executor.tools();
        assert_eq!(tools.len(), 1, "应该注册了一个工具");
        assert_eq!(tools[0].name, "test-tool");
    }

    #[test]
    fn 注册多个工具() {
        let executor = ToolExecutor::new();
        let executor = executor.register(MockTool::success("tool-1"));
        let executor = executor.register(MockTool::success("tool-2"));
        let executor = executor.register(MockTool::success("tool-3"));

        let tools = executor.tools();
        assert_eq!(tools.len(), 3, "应该注册了三个工具");
    }

    #[test]
    fn 获取已注册的工具() {
        let executor = ToolExecutor::new();
        let executor = executor.register(MockTool::success("test-tool"));

        let tool = executor.get("test-tool");
        assert!(tool.is_some(), "应该能获取到已注册的工具");

        let missing = executor.get("nonexistent");
        assert!(missing.is_none(), "不应该获取到不存在的工具");
    }

    #[test]
    fn 同名工具会被覆盖() {
        let executor = ToolExecutor::new();
        let executor = executor.register(MockTool::success("tool"));
        let executor = executor.register(MockTool::success("tool")); // 同名

        let tools = executor.tools();
        assert_eq!(tools.len(), 1, "同名工具应该被覆盖");
    }
}

mod tool_execution {
    use super::*;

    #[tokio::test]
    async fn 执行单个工具调用() {
        let executor = ToolExecutor::new();
        let executor = executor.register(MockTool::success("test-tool"));

        let tool_calls = vec![ToolCall {
            id: "call-1".to_string(),
            name: "test-tool".to_string(),
            arguments: serde_json::json!({}),
        }];

        let results = executor.execute(tool_calls, new_context()).await;
        assert!(results.is_ok(), "执行应该成功");

        let results = results.unwrap();
        assert_eq!(results.len(), 1, "应该有一个结果");
        assert!(results[0].output.contains("Mock tool executed"));
    }

    #[tokio::test]
    async fn 执行多个工具调用() {
        let executor = ToolExecutor::new();
        let executor = executor.register(MockTool::success("tool-1"));
        let executor = executor.register(MockTool::success("tool-2"));

        let tool_calls = vec![
            ToolCall {
                id: "call-1".to_string(),
                name: "tool-1".to_string(),
                arguments: serde_json::json!({}),
            },
            ToolCall {
                id: "call-2".to_string(),
                name: "tool-2".to_string(),
                arguments: serde_json::json!({}),
            },
        ];

        let results = executor.execute(tool_calls, new_context()).await.unwrap();
        assert_eq!(results.len(), 2, "应该有两个结果");
    }

    #[tokio::test]
    async fn 工具执行失败返回错误结果() {
        let executor = ToolExecutor::new();
        let executor = executor.register(MockTool::failing("failing-tool"));

        let tool_calls = vec![ToolCall {
            id: "call-1".to_string(),
            name: "failing-tool".to_string(),
            arguments: serde_json::json!({}),
        }];

        let results = executor.execute(tool_calls, new_context()).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].output.contains("Tool execution failed"));
    }

    #[tokio::test]
    async fn 调用不存在的工具() {
        let executor = ToolExecutor::new();

        let tool_calls = vec![ToolCall {
            id: "call-1".to_string(),
            name: "nonexistent".to_string(),
            arguments: serde_json::json!({}),
        }];

        let results = executor.execute(tool_calls, new_context()).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].output.contains("not registered"));
    }

    #[tokio::test]
    async fn 空工具调用列表() {
        let executor = ToolExecutor::new();

        let results = executor.execute(vec![], new_context()).await.unwrap();
        assert!(results.is_empty(), "空调用列表应该返回空结果");
    }
}

mod executor_immutability {
    use super::*;

    #[test]
    fn register_返回新的执行器实例() {
        // register 拿走所有权，返回新的执行器
        let executor = ToolExecutor::new();
        let executor = executor.register(MockTool::success("tool"));

        // 返回的执行器有工具
        assert_eq!(executor.tools().len(), 1, "新执行器应该有工具");
    }

    #[test]
    fn 执行器可以克隆() {
        let executor = ToolExecutor::new();
        let executor = executor.register(MockTool::success("tool"));

        let cloned = executor.clone();
        assert_eq!(cloned.tools().len(), 1);
    }
}