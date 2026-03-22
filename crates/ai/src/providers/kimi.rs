use crate::models::Api;
use crate::types::{ContentBlock, Message, Role, StopReason, ToolCall, Usage};
use crate::{Context, StreamEvent};
use crate::provider::Provider;
use crate::ProviderError;
use crate::stream::StreamResponse;
use crate::utils::parse_streaming_json;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio::sync::mpsc;
use std::io::ErrorKind;
use tracing::debug;

const KIMI_API_URL: &str = "https://api.kimi.com/coding/v1/messages";

pub struct KimiProvider {
    client: Client,
    model: String,
    api_key: String,
    base_url: String,
    thinking_budget: Option<u32>,
}

impl KimiProvider {
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            model: model.into(),
            api_key: api_key.into(),
            base_url: KIMI_API_URL.to_string(),
            thinking_budget: None,
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_thinking(mut self, budget_tokens: u32) -> Self {
        self.thinking_budget = Some(budget_tokens);
        self
    }
}

#[derive(Debug, Serialize)]
struct KimiRequest<'a> {
    model: &'a str,
    messages: Vec<KimiMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<KimiTool<'a>>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<KimiThinking>,
}

#[derive(Debug, Serialize)]
struct KimiThinking {
    #[serde(rename = "type")]
    thinking_type: String,
    budget_tokens: u32,
}

#[derive(Debug, Serialize)]
struct KimiMessage<'a> {
    role: &'a str,
    content: Vec<KimiContent<'a>>,
}

#[derive(Debug, Serialize)]
struct KimiContentText<'a> {
    #[serde(rename = "type")]
    content_type: &'a str,
    text: &'a str,
}

#[derive(Debug, Serialize)]
struct KimiContentImage {
    #[serde(rename = "type")]
    content_type: String,
    image_url: KimiImageUrl,
}

#[derive(Debug, Serialize)]
struct KimiImageUrl {
    url: String,
}

#[derive(Debug, Serialize)]
struct KimiToolResult<'a> {
    #[serde(rename = "type")]
    content_type: &'a str,
    #[serde(rename = "tool_use_id")]
    tool_use_id: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct KimiThinkingContent<'a> {
    #[serde(rename = "type")]
    content_type: &'a str,
    thinking: &'a str,
}

#[derive(Debug, Serialize)]
struct KimiToolUse<'a> {
    #[serde(rename = "type")]
    content_type: &'a str,
    id: &'a str,
    name: &'a str,
    input: &'a serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<&'a str>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum KimiContent<'a> {
    Text(KimiContentText<'a>),
    ImageUrl(KimiContentImage),
    ToolUse(KimiToolUse<'a>),
    ToolResult(KimiToolResult<'a>),
    Thinking(KimiThinkingContent<'a>),
}

#[derive(Debug, Serialize)]
struct KimiTool<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct KimiStreamEventMessageStart {
    message: KimiMessageStart,
}

#[derive(Debug, Deserialize)]
struct KimiStreamEventMessageDelta {
    delta: KimiMessageDelta,
    usage: Option<KimiUsage>,
}

#[derive(Debug, Deserialize)]
struct KimiStreamEventContentBlockStart {
    index: usize,
    content_block: KimiContentBlockStart,
}

#[derive(Debug, Deserialize)]
struct KimiStreamEventContentBlockDelta {
    index: usize,
    delta: KimiContentBlockDeltaInner,
}

#[derive(Debug, Deserialize)]
struct KimiStreamEventContentBlockStop {
    index: usize,
}

#[derive(Debug, Deserialize)]
struct KimiStreamEventError {
    #[allow(dead_code)]
    code: String,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum KimiStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart(KimiStreamEventMessageStart),
    #[serde(rename = "message_delta")]
    MessageDelta(KimiStreamEventMessageDelta),
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "content_block_start")]
    ContentBlockStart(KimiStreamEventContentBlockStart),
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta(KimiStreamEventContentBlockDelta),
    #[serde(rename = "content_block_stop")]
    ContentBlockStop(KimiStreamEventContentBlockStop),
    #[serde(rename = "error")]
    Error(KimiStreamEventError),
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Debug, Deserialize)]
struct KimiMessageStart {
    id: String,
    #[allow(dead_code)]
    role: String,
    #[allow(dead_code)]
    content: Vec<serde_json::Value>,
    usage: Option<KimiUsage>,
}

#[derive(Debug, Deserialize)]
struct KimiMessageDelta {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KimiUsage {
    #[serde(rename = "input_tokens")]
    input_tokens: u64,
    #[serde(rename = "output_tokens")]
    output_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct KimiContentBlockStart {
    #[serde(rename = "type")]
    content_type: String,
    name: Option<String>,
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KimiContentBlockDeltaInner {
    #[serde(rename = "type")]
    delta_type: String,
    text: Option<String>,
    #[serde(rename = "partial_json")]
    partial_json: Option<String>,
    #[serde(rename = "thinking")]
    thinking: Option<String>,
}

fn convert_message_to_kimi(message: &Message) -> Option<KimiMessage<'_>> {
    match message.role {
        Role::User => {
            let content: Vec<KimiContent> = message.content.iter().filter_map(|block| {
                match block {
                    ContentBlock::Text(t) => Some(KimiContent::Text(KimiContentText {
                        content_type: "text",
                        text: &t.text,
                    })),
                    ContentBlock::Image(img) => Some(KimiContent::ImageUrl(KimiContentImage {
                        content_type: "image_url".to_string(),
                        image_url: KimiImageUrl {
                            url: format!("data:{};base64,{}", img.mime_type, img.data),
                        },
                    })),
                    _ => None,
                }
            }).collect();
            Some(KimiMessage { role: "user", content })
        }
        Role::Assistant => {
            let content: Vec<KimiContent> = message.content.iter().filter_map(|block| {
                match block {
                    ContentBlock::Text(t) => Some(KimiContent::Text(KimiContentText {
                        content_type: "text",
                        text: &t.text,
                    })),
                    ContentBlock::ToolCall(tc) => {
                        debug!(id = %tc.id, name = %tc.name, has_reasoning = tc.reasoning_content.is_some(), "ToolCall");
                        Some(KimiContent::ToolUse(KimiToolUse {
                            content_type: "tool_use",
                            id: &tc.id,
                            name: &tc.name,
                            input: &tc.arguments,
                            reasoning_content: tc.reasoning_content.as_deref(),
                        }))
                    }
                    ContentBlock::Thinking(t) => Some(KimiContent::Thinking(KimiThinkingContent {
                        content_type: "thinking",
                        thinking: &t.thinking,
                    })),
                    _ => None,
                }
            }).collect();
            Some(KimiMessage { role: "assistant", content })
        }
        Role::ToolResult => {
            let tool_call_id = message.tool_call_id.as_deref().unwrap_or("");
            debug!(tool_call_id = %tool_call_id, name = ?message.name, "ToolResult");
            let content: Vec<KimiContent> = message.content.iter().filter_map(|block| {
                if let ContentBlock::Text(t) = block {
                    Some(KimiContent::ToolResult(KimiToolResult {
                        content_type: "tool_result",
                        tool_use_id: tool_call_id,
                        content: &t.text,
                    }))
                } else {
                    None
                }
            }).collect();
            Some(KimiMessage { role: "user", content })
        }
        Role::System => None,
    }
}

fn convert_tool(tool: &crate::types::Tool) -> KimiTool<'_> {
    KimiTool {
        name: &tool.name,
        description: &tool.description,
        parameters: &tool.parameters,
    }
}

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

struct SseStream<S> {
    inner: S,
    buffer: bytes::Bytes,
    current_event: Option<String>,
    current_data: String,
}

impl<S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin> SseStream<S> {
    fn new(stream: S) -> Self {
        Self {
            inner: stream,
            buffer: bytes::Bytes::new(),
            current_event: None,
            current_data: String::new(),
        }
    }
}

impl<S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin> Stream for SseStream<S> {
    type Item = Result<SseEvent, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;
        use bytes::Buf;

        loop {
            if let Some(pos) = find_line_end(&self.buffer) {
                let line = String::from_utf8_lossy(&self.buffer[..pos]).to_string();
                self.buffer.advance(pos + 1);

                if line.is_empty() {
                    if !self.current_data.is_empty() || self.current_event.is_some() {
                        let event = SseEvent {
                            event: self.current_event.take(),
                            data: std::mem::take(&mut self.current_data),
                            id: None,
                        };
                        return Poll::Ready(Some(Ok(event)));
                    }
                    continue;
                }

                if let Some(colon_pos) = line.find(':') {
                    let field = &line[..colon_pos];
                    let value = line[colon_pos + 1..].trim_start().to_string();

                    match field as &str {
                        "event" => {
                            self.current_event = Some(value);
                        }
                        "data" => {
                            if self.current_data.is_empty() {
                                self.current_data = value;
                            } else {
                                self.current_data.push('\n');
                                self.current_data.push_str(&value);
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                match Pin::new(&mut self.inner).poll_next(cx) {
                    Poll::Ready(Some(Ok(chunk))) => {
                        let mut combined = self.buffer.to_vec();
                        combined.extend_from_slice(&chunk);
                        self.buffer = bytes::Bytes::from(combined);
                    }
                    Poll::Ready(Some(Err(e))) => {
                        return Poll::Ready(Some(Err(std::io::Error::new(ErrorKind::Other, e.to_string()))));
                    }
                    Poll::Ready(None) => {
                        if self.buffer.is_empty() && self.current_data.is_empty() && self.current_event.is_none() {
                            return Poll::Ready(None);
                        }
                        if !self.current_data.is_empty() || self.current_event.is_some() {
                            let event = SseEvent {
                                event: self.current_event.take(),
                                data: std::mem::take(&mut self.current_data),
                                id: None,
                            };
                            self.buffer = bytes::Bytes::new();
                            return Poll::Ready(Some(Ok(event)));
                        }
                        return Poll::Ready(None);
                    }
                    Poll::Pending => return Poll::Pending,
                }
            }
        }
    }
}

fn find_line_end(buf: &bytes::Bytes) -> Option<usize> {
    for (i, &b) in buf.iter().enumerate() {
        if b == b'\n' || b == b'\r' {
            return Some(i);
        }
    }
    None
}

#[async_trait]
impl Provider for KimiProvider {
    fn name(&self) -> &str {
        "kimi-coding"
    }

    fn api(&self) -> Api {
        Api::KimiCoding
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    async fn stream(&self, context: &Context) -> anyhow::Result<StreamResponse> {
        let messages: Vec<KimiMessage> = context.messages.iter()
            .filter_map(convert_message_to_kimi)
            .collect();

        let tools = if context.tools.is_empty() {
            None
        } else {
            Some(context.tools.iter().map(convert_tool).collect())
        };

        let system = context.system_prompt.as_deref();
        
        let thinking = self.thinking_budget.map(|budget| KimiThinking {
            thinking_type: "enabled".to_string(),
            budget_tokens: budget,
        });

        let request = KimiRequest {
            model: &self.model,
            messages,
            system,
            max_tokens: Some(8192),
            temperature: None,
            tools,
            stream: true,
            thinking,
        };

        debug!(request = %serde_json::to_string_pretty(&request).unwrap_or_default(), "REQUEST");

        let req_builder = self
            .client
            .post(&self.base_url)
            .header("X-API-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("anthropic-version", "2023-06-01");

        let response = req_builder.json(&request).send().await?;
        let status = response.status();
        if !(200..300).contains(&status.as_u16()) {
            let body = response.text().await.unwrap_or_else(|_| "".to_string());
            return Err(ProviderError::Provider(format!(
                "Kimi API error (HTTP {}): {}",
                status, body
            )).into());
        }

        let (tx, rx) = mpsc::channel(100);

        let model = self.model.clone();
        let provider = self.name().to_string();

        tokio::spawn(async move {
            let mut event_source = SseStream::new(response.bytes_stream());

            let mut partial = crate::types::AssistantMessage {
                role: Role::Assistant,
                content: Vec::new(),
                model: model.clone(),
                provider: provider.clone(),
                usage: Usage::default(),
                stop_reason: StopReason::Stop,
                response_id: None,
                error_message: None,
            };

            let mut current_tool_json = String::new();
            let mut current_thinking = String::new();
            let mut done = false;

            let _ = tx.send(Ok(StreamEvent::Start { partial: partial.clone() })).await;

            while let Some(result) = event_source.next().await {
                match result {
                    Ok(sse_event) => {
                        if sse_event.event.as_deref() == Some("ping") {
                            continue;
                        }

                        let data = &sse_event.data;
                        if data.is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<KimiStreamEvent>(data) {
                            Ok(event) => {
                                debug!(data = %data, "SSE");
                                match event {
                                    KimiStreamEvent::MessageStart(inner) => {
                                        partial.response_id = Some(inner.message.id);
                                        if let Some(usage) = inner.message.usage {
                                            partial.usage.input_tokens = usage.input_tokens;
                                            partial.usage.output_tokens = usage.output_tokens;
                                        }
                                    }
                                    KimiStreamEvent::ContentBlockStart(inner) => {
                                        debug!(index = inner.index, type = %inner.content_block.content_type, id = ?inner.content_block.id, name = ?inner.content_block.name, "ContentBlockStart");
                                        match inner.content_block.content_type.as_str() {
                                            "text" => {
                                                partial.content.push(ContentBlock::Text(crate::types::TextContent {
                                                    text: String::new(),
                                                }));
                                                let _ = tx.send(Ok(StreamEvent::TextStart { content_index: inner.index })).await;
                                            }
                                            "tool_use" => {
                                                let id = inner.content_block.id.clone().unwrap_or_else(|| format!("tool_{}", inner.index));
                                                let name = inner.content_block.name.clone().unwrap_or_default();
                                                debug!(id = %id, "tool_use");
                                                current_tool_json.clear();
                                                let reasoning = if current_thinking.is_empty() {
                                                    debug!("current_thinking is EMPTY");
                                                    None
                                                } else {
                                                    debug!(thinking_len = current_thinking.len(), "using thinking");
                                                    Some(current_thinking.clone())
                                                };
                                                partial.content.push(ContentBlock::ToolCall(ToolCall {
                                                    id: id.clone(),
                                                    name: name.clone(),
                                                    arguments: serde_json::Value::Null,
                                                    reasoning_content: reasoning,
                                                }));
                                                let _ = tx.send(Ok(StreamEvent::ToolCallStart { content_index: inner.index })).await;
                                            }
                                            "thinking" => {
                                                current_thinking.clear();
                                                partial.content.push(ContentBlock::Thinking(crate::types::ThinkingContent {
                                                    thinking: String::new(),
                                                    thinking_signature: None,
                                                    redacted: None,
                                                }));
                                                let _ = tx.send(Ok(StreamEvent::ThinkingStart { content_index: inner.index })).await;
                                            }
                                            _ => {}
                                        }
                                    }
                                    KimiStreamEvent::ContentBlockDelta(inner) => {
                                        match inner.delta.delta_type.as_str() {
                                            "text_delta" => {
                                                if let Some(text) = &inner.delta.text {
                                                    if let Some(ContentBlock::Text(t)) = partial.content.get_mut(inner.index) {
                                                        t.text.push_str(text);
                                                    }
                                                    let _ = tx.send(Ok(StreamEvent::TextDelta { content_index: inner.index, delta: text.clone() })).await;
                                                }
                                            }
                                            "input_json_delta" => {
                                                current_tool_json.push_str(inner.delta.partial_json.as_deref().unwrap_or(""));
                                                let _ = tx.send(Ok(StreamEvent::ToolCallDelta { content_index: inner.index, delta: inner.delta.partial_json.clone().unwrap_or_default() })).await;
                                            }
                                            "thinking_delta" => {
                                                if let Some(thinking) = &inner.delta.thinking {
                                                    if let Some(ContentBlock::Thinking(t)) = partial.content.get_mut(inner.index) {
                                                        t.thinking.push_str(thinking);
                                                    }
                                                    current_thinking.push_str(thinking);
                                                    let _ = tx.send(Ok(StreamEvent::ThinkingDelta { content_index: inner.index, delta: thinking.clone() })).await;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    KimiStreamEvent::ContentBlockStop(inner) => {
                                        debug!(index = inner.index, "ContentBlockStop");
                                        if let Some(ContentBlock::ToolCall(tc)) = partial.content.get_mut(inner.index) {
                                            debug!(id = %tc.id, name = %tc.name, args = %current_tool_json, has_reasoning = tc.reasoning_content.is_some(), "ToolCallEnd");
                                            let _args_str = current_tool_json.clone();
                                            tc.arguments = parse_streaming_json(&current_tool_json);

                                            let _ = tx.send(Ok(StreamEvent::ToolCallEnd {
                                                content_index: inner.index,
                                                tool_call: tc.clone(),
                                            })).await;
                                        }
                                    }
                                    KimiStreamEvent::MessageDelta(inner) => {
                                        if let Some(reason) = inner.delta.stop_reason {
                                            partial.stop_reason = match reason.as_str() {
                                                "stop" => StopReason::Stop,
                                                "length" => StopReason::Length,
                                                "tool_calls" => StopReason::ToolUse,
                                                _ => StopReason::Stop,
                                            };
                                        }
                                        if let Some(usage) = inner.usage {
                                            partial.usage.output_tokens = usage.output_tokens;
                                        }
                                    }
                                    KimiStreamEvent::MessageStop => {
                                        done = true;
                                        let _ = tx.send(Ok(StreamEvent::Done { reason: partial.stop_reason, message: partial.clone() })).await;
                                    }
                                    KimiStreamEvent::Error(inner) => {
                                        done = true;
                                        partial.stop_reason = StopReason::Error;
                                        partial.error_message = Some(inner.message.clone());
                                        let _ = tx.send(Ok(StreamEvent::Error { reason: StopReason::Error, error: inner.message })).await;
                                    }
                                    KimiStreamEvent::Ping => {}
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(Err(anyhow::anyhow!(
                                    "Failed to parse SSE event: {} - data: {}",
                                    e, data
                                ))).await;
                            }
                        }

                        if done {
                            break;
                        }
                    }
                    Err(e) => {
                        if e.kind() == ErrorKind::WriteZero {
                            continue;
                        }
                        let _ = tx.send(Err(anyhow::anyhow!("Stream error: {}", e))).await;
                        break;
                    }
                }
            }
        });

        Ok(StreamResponse::new(rx))
    }
}
