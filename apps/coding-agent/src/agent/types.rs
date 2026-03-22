//! Agent 核心类型定义
//!
//! 定义 Agent 系统中使用的核心数据结构。

use ai::types::Message;
use serde::{Deserialize, Serialize};

// ============================================================================
// 工具执行上下文
// ============================================================================

/// 工具执行时的上下文信息
///
/// 包含会话信息、消息 ID、Agent 名称，以及中止信号。
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// 当前会话的唯一标识
    pub session_id: String,
    /// 当前消息的唯一标识
    pub message_id: String,
    /// Agent 的名称
    pub agent_name: String,
    /// 中止信号，用于取消长时间运行的操作
    pub abort: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

// ============================================================================
// 附件
// ============================================================================

/// 附件
///
/// 用于在工具结果中附加额外的二进制数据（如图片、文件等）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// MIME 类型，如 "image/png"、"application/json"
    pub mime: String,
    /// 附件内容（通常是 Base64 编码）
    pub data: String,
}

// ============================================================================
// 工具结果
// ============================================================================

/// 工具执行结果
///
/// 当工具执行完成后，返回此结构描述执行结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 结果的简短标题
    pub title: String,
    /// 执行输出内容
    pub output: String,
    /// 额外元数据（如 exit_code、additions、deletions 等）
    pub metadata: serde_json::Value,
    /// 附加的文件或数据
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

impl ToolResult {
    /// 创建新的工具结果
    pub fn new(title: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            output: output.into(),
            metadata: serde_json::json!({}),
            attachments: Vec::new(),
        }
    }

    /// 添加元数据（链式调用）
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// 添加附件（链式调用）
    pub fn with_attachment(mut self, mime: impl Into<String>, data: impl Into<String>) -> Self {
        self.attachments.push(Attachment {
            mime: mime.into(),
            data: data.into(),
        });
        self
    }
}

// ============================================================================
// 工具调用
// ============================================================================

/// 工具调用请求
///
/// 描述 Agent 请求调用某个工具的信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 调用 ID，用于匹配响应
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 工具参数（JSON 格式）
    pub arguments: serde_json::Value,
}

/// 工具定义
///
/// 描述工具的名称、用途和参数规范。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefine {
    /// 工具唯一名称
    pub name: String,
    /// 工具功能描述
    pub description: String,
    /// JSON Schema 格式的参数定义
    pub parameters: serde_json::Value,
}

// ============================================================================
// Agent 事件
// ============================================================================

/// Agent 事件
///
/// 描述 Agent 运行过程中发生的各种事件，用于事件广播和日志记录。
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent 开始运行
    AgentStart,

    /// Agent 运行结束
    AgentEnd {
        /// 最终的消息历史
        messages: Vec<Message>,
    },

    /// 对话轮次开始
    TurnStart,

    /// 对话轮次结束
    TurnEnd {
        /// Assistant 消息
        message: Message,
        /// 工具执行结果
        tool_results: Vec<Message>,
    },

    /// 开始接收消息
    MessageStart { message: Message },

    /// 文本内容片段
    TextDelta { content_index: usize, delta: String },

    /// 文本内容结束
    TextEnd {
        content_index: usize,
        content: Message,
    },

    /// 开始思考
    ThinkingStart { content_index: usize },

    /// 思考内容片段
    ThinkingDelta { content_index: usize, delta: String },

    /// 思考结束
    ThinkingEnd {
        content_index: usize,
        content: Message,
    },

    /// 开始接收工具调用
    ToolCallStart { content_index: usize },

    /// 工具调用参数片段
    ToolCallDelta { content_index: usize, delta: String },

    /// 工具调用结束
    ToolCallEnd {
        content_index: usize,
        tool_call: ToolCall,
    },

    /// 消息结束
    MessageEnd { message: Message },

    /// 开始执行工具
    ToolExecutionStart {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    },

    /// 工具执行进度更新
    ToolExecutionUpdate {
        tool_call_id: String,
        partial_result: String,
    },

    /// 工具执行结束
    ToolExecutionEnd {
        tool_call_id: String,
        tool_name: String,
        result: serde_json::Value,
        is_error: bool,
    },

    /// 发生错误
    Error { error: String },
}

// ============================================================================
// Agent 状态
// ============================================================================

/// Agent 状态
///
/// 记录 Agent 运行过程中的持久状态。
#[derive(Debug, Clone, Default)]
pub struct AgentState {
    /// 消息历史
    pub messages: Vec<Message>,
    /// 可用工具定义列表
    pub tools: Vec<ToolDefine>,
    /// 系统提示词
    pub system_prompt: String,
}
