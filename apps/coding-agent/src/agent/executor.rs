//! 工具执行器
//! 
//! 负责管理 Agent 可用的工具，并执行工具调用。

use super::types::{ToolCall, ToolContext, ToolDefine, ToolResult};
use anyhow::Result;
use std::pin::Pin;
use std::sync::Arc;

/// 工具接口
/// 
/// 所有 Agent 工具都必须实现此接口。
/// 工具提供两个核心能力：
/// - `define`: 描述工具的名称、用途和参数
/// - `execute`: 执行工具逻辑并返回结果
pub trait Tool: Send + Sync {
    /// 返回工具的定义信息
    fn define(&self) -> ToolDefine;
    
    /// 执行工具
    fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult>> + Send + '_>>;
}

/// 工具执行器
/// 
/// 管理一组工具，提供注册、查找和批量执行能力。
/// 执行器是不可变的：每次注册新工具都返回新的实例。
#[derive(Clone)]
pub struct ToolExecutor {
    /// 工具注册表，使用 Arc 实现共享所有权
    tools: Arc<std::collections::HashMap<String, Arc<dyn Tool>>>,
}

impl ToolExecutor {
    /// 创建一个空的工具执行器
    pub fn new() -> Self {
        Self {
            tools: Arc::new(std::collections::HashMap::new()),
        }
    }

    /// 注册一个工具
    /// 
    /// 返回一个新的执行器实例，保留原执行器的功能。
    /// 如果已存在同名工具，会被替换。
    pub fn register<T: Tool + 'static>(self, tool: T) -> Self {
        let name = tool.define().name.clone();
        
        // 从当前工具表克隆，插入新工具
        let mut new_tools = (*self.tools).clone();
        new_tools.insert(name, Arc::new(tool));
        
        Self {
            tools: Arc::new(new_tools),
        }
    }

    /// 根据名称查找工具
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// 返回所有已注册工具的定义
    pub fn tools(&self) -> Vec<ToolDefine> {
        self.tools.values().map(|t| t.define()).collect()
    }

    /// 批量执行工具调用
    /// 
    /// 按顺序执行每个工具调用，返回对应数量的结果。
    /// 如果某个工具执行失败，结果中会包含错误信息，而不是整个执行失败。
    pub async fn execute(
        &self,
        tool_calls: Vec<ToolCall>,
        ctx: ToolContext,
    ) -> Result<Vec<ToolResult>> {
        let mut results = Vec::with_capacity(tool_calls.len());

        for tool_call in tool_calls {
            let result = match self.get(&tool_call.name) {
                Some(tool) => {
                    // 工具存在，执行它
                    match tool.execute(tool_call.arguments.clone(), ctx.clone()).await {
                        Ok(r) => r,
                        Err(e) => ToolResult::new(
                            format!("执行错误: {}", e),
                            format!("Tool execution failed: {}", e),
                        ),
                    }
                }
                None => {
                    // 工具不存在，返回友好错误
                    ToolResult::new(
                        format!("工具未找到: {}", tool_call.name),
                        format!("Tool '{}' is not registered", tool_call.name),
                    )
                }
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