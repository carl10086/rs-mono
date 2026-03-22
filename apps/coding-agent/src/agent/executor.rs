use super::types::{ToolCall, ToolContext, ToolDefine, ToolResult};
use anyhow::Result;
use std::pin::Pin;
use std::sync::Arc;

pub trait Tool: Send + Sync {
    fn define(&self) -> ToolDefine;
    fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult>> + Send + '_>>;
}

#[derive(Clone)]
pub struct ToolExecutor {
    tools: Arc<std::collections::HashMap<String, Arc<dyn Tool>>>,
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(std::collections::HashMap::new()),
        }
    }

    pub fn register<T: Tool + 'static>(self, tool: T) -> Self {
        let mut new_tools = (*self.tools).clone();
        let name = tool.define().name.clone();
        new_tools.insert(name, Arc::new(tool));
        Self {
            tools: Arc::new(new_tools),
        }
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn tools(&self) -> Vec<ToolDefine> {
        self.tools.values().map(|t| t.define()).collect()
    }

    pub async fn execute(
        &self,
        tool_calls: Vec<ToolCall>,
        ctx: ToolContext,
    ) -> Result<Vec<ToolResult>> {
        let mut results = Vec::new();

        for tool_call in tool_calls {
            let result = match self.get(&tool_call.name) {
                Some(tool) => {
                    match tool.execute(tool_call.arguments.clone(), ctx.clone()).await {
                        Ok(r) => r,
                        Err(e) => ToolResult::new(
                            format!("Error: {}", e),
                            format!("Tool execution failed: {}", e),
                        ),
                    }
                }
                None => ToolResult::new(
                    format!("Tool not found: {}", tool_call.name),
                    format!("Tool '{}' is not registered", tool_call.name),
                ),
            };
            results.push(result);
        }

        Ok(results)
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}
