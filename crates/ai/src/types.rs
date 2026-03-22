use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    #[serde(rename = "toolResult")]
    ToolResult,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text(TextContent),
    #[serde(rename = "thinking")]
    Thinking(ThinkingContent),
    Image(ImageContent),
    #[serde(rename = "toolCall")]
    ToolCall(ToolCall),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingContent {
    pub thinking: String,
    #[serde(default)]
    pub thinking_signature: Option<String>,
    #[serde(default)]
    pub redacted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    pub data: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageCost {
    pub input: f64,
    pub output: f64,
    #[serde(default)]
    pub cache_read: f64,
    #[serde(default)]
    pub cache_write: f64,
    #[serde(default)]
    pub total: f64,
}

impl UsageCost {
    pub fn new() -> Self {
        Self {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
            total: 0.0,
        }
    }

    pub fn calculate(&mut self) {
        self.total = self.input + self.output + self.cache_read + self.cache_write;
    }
}

impl Default for UsageCost {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub cache_write_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<UsageCost>,
}

impl Usage {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StopReason {
    Stop,
    Length,
    #[serde(rename = "toolUse")]
    ToolUse,
    Error,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub provider: String,
    pub usage: Usage,
    pub stop_reason: StopReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

impl ThinkingLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThinkingLevel::Minimal => "minimal",
            ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "medium",
            ThinkingLevel::High => "high",
            ThinkingLevel::Xhigh => "xhigh",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ThinkingBudgets {
    pub minimal: Option<u32>,
    pub low: Option<u32>,
    pub medium: Option<u32>,
    pub high: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub system_prompt: Option<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub thinking: Option<ThinkingLevel>,
    pub thinking_budgets: Option<ThinkingBudgets>,
    pub provider_options: Option<serde_json::Value>,
}

impl Default for Usage {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost: None,
        }
    }
}

impl Default for StopReason {
    fn default() -> Self {
        StopReason::Stop
    }
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::ToolResult => "tool_result",
            Role::System => "system",
        }
    }
}

impl ContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        ContentBlock::Text(TextContent { text: text.into() })
    }

    pub fn thinking(content: impl Into<String>) -> Self {
        ContentBlock::Thinking(ThinkingContent {
            thinking: content.into(),
            thinking_signature: None,
            redacted: None,
        })
    }

    pub fn tool_call(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        ContentBlock::ToolCall(ToolCall {
            id: id.into(),
            name: name.into(),
            arguments,
            reasoning_content: None,
        })
    }
}

impl Message {
    pub fn user(content: impl Into<Vec<ContentBlock>>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            name: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<Vec<ContentBlock>>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            name: None,
            tool_call_id: None,
        }
    }

    pub fn tool_result(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<Vec<ContentBlock>>,
    ) -> Self {
        Self {
            role: Role::ToolResult,
            content: content.into(),
            name: Some(name.into()),
            tool_call_id: Some(tool_call_id.into()),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: vec![ContentBlock::Text(TextContent {
                text: content.into(),
            })],
            name: None,
            tool_call_id: None,
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        for block in &self.content {
            if let ContentBlock::Text(t) = block {
                return Some(&t.text);
            }
        }
        None
    }
}

impl Context {
    pub fn new() -> Self {
        Self {
            system_prompt: None,
            messages: Vec::new(),
            tools: Vec::new(),
            max_tokens: None,
            temperature: None,
            thinking: None,
            thinking_budgets: None,
            provider_options: None,
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools.extend(tools);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_thinking(mut self, thinking: ThinkingLevel) -> Self {
        self.thinking = Some(thinking);
        self
    }

    pub fn with_thinking_budgets(mut self, budgets: ThinkingBudgets) -> Self {
        self.thinking_budgets = Some(budgets);
        self
    }

    pub fn with_provider_options(mut self, options: serde_json::Value) -> Self {
        self.provider_options = Some(options);
        self
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_user() {
        let msg = Message::user(vec![ContentBlock::text("Hello, world!")]);
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.text_content(), Some("Hello, world!"));
    }

    #[test]
    fn test_message_assistant() {
        let msg = Message::assistant(vec![ContentBlock::text("Hi there!")]);
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.text_content(), Some("Hi there!"));
    }

    #[test]
    fn test_message_tool_result() {
        let msg = Message::tool_result("tool_123", "bash", vec![ContentBlock::text("/home/user")]);
        assert_eq!(msg.role, Role::ToolResult);
        assert_eq!(msg.tool_call_id, Some("tool_123".to_string()));
        assert_eq!(msg.name, Some("bash".to_string()));
    }

    #[test]
    fn test_message_system() {
        let msg = Message::system("You are helpful.");
        assert_eq!(msg.role, Role::System);
        assert_eq!(msg.text_content(), Some("You are helpful."));
    }

    #[test]
    fn test_context_builder() {
        let ctx = Context::new()
            .with_system_prompt("You are a helpful assistant.")
            .with_message(Message::user(vec![ContentBlock::text("Hello")]))
            .with_message(Message::assistant(vec![ContentBlock::text("Hi!")]))
            .with_max_tokens(1024)
            .with_temperature(0.7);

        assert_eq!(
            ctx.system_prompt,
            Some("You are a helpful assistant.".to_string())
        );
        assert_eq!(ctx.messages.len(), 2);
        assert_eq!(ctx.max_tokens, Some(1024));
        assert_eq!(ctx.temperature, Some(0.7));
    }

    #[test]
    fn test_content_block_helpers() {
        let text = ContentBlock::text("Hello");
        assert!(matches!(text, ContentBlock::Text(_)));

        let thinking = ContentBlock::thinking("Let me think...");
        assert!(matches!(thinking, ContentBlock::Thinking(_)));

        let tool_call = ContentBlock::tool_call("tool_1", "bash", serde_json::json!({"cmd": "ls"}));
        assert!(matches!(tool_call, ContentBlock::ToolCall(_)));
    }

    #[test]
    fn test_usage_total_tokens() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost: None,
        };
        assert_eq!(usage.total_tokens(), 150);
    }

    #[test]
    fn test_usage_cost() {
        let mut cost = UsageCost::new();
        cost.input = 0.001;
        cost.output = 0.002;
        cost.cache_read = 0.0005;
        cost.cache_write = 0.0001;
        cost.calculate();
        assert!((cost.total - 0.0036).abs() < 0.0001);
    }
}
