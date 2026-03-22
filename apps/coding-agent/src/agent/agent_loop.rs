use super::executor::{Tool, ToolExecutor};
use super::types::{AgentEvent, AgentState, ToolCall, ToolContext};
use super::event::{EventBroadcaster, EventHandler, SubscriptionId};
use ai::types::{ContentBlock, Message, ToolCall as AiToolCall};
use ai::{client, StreamEvent};
use anyhow::{anyhow, Result};
use futures::StreamExt;
use std::sync::Arc;

/// AgentLoop 配置
#[derive(Clone)]
pub struct AgentLoopConfig {
    /// 使用的模型
    pub model: ai::Model,
    /// 是否启用推理
    pub reasoning: Option<ai::ThinkingLevel>,
    /// 系统提示词
    pub system_prompt: String,
}

impl AgentLoopConfig {
    pub fn new(model: ai::Model) -> Self {
        Self {
            model,
            reasoning: None,
            system_prompt: String::new(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    pub fn with_reasoning(mut self, reasoning: ai::ThinkingLevel) -> Self {
        self.reasoning = Some(reasoning);
        self
    }
}

/// Agent 主循环
/// 负责管理 LLM 对话、工具执行和事件广播
pub struct AgentLoop {
    /// 配置
    config: AgentLoopConfig,
    /// 状态（消息历史、工具定义等）
    state: AgentState,
    /// 工具执行器
    executor: ToolExecutor,
    /// 事件广播器
    broadcaster: EventBroadcaster,
}

impl AgentLoop {
    /// 创建新的 AgentLoop
    pub fn new(config: AgentLoopConfig) -> Self {
        Self {
            config,
            state: AgentState::default(),
            executor: ToolExecutor::new(),
            broadcaster: EventBroadcaster::new(),
        }
    }

    /// 获取广播器当前订阅的处理器数量
    pub fn broadcaster_handler_count(&self) -> usize {
        self.broadcaster.handler_count()
    }

    /// 订阅事件处理器
    pub fn subscribe<H: EventHandler + 'static>(&mut self, handler: H) -> SubscriptionId {
        self.broadcaster.subscribe(handler)
    }

    /// 取消订阅
    pub fn unsubscribe(&mut self, id: SubscriptionId) {
        self.broadcaster.unsubscribe(id);
    }

    /// 广播事件到所有订阅的处理器
    pub fn broadcast(&self, event: &AgentEvent) {
        self.broadcaster.broadcast(event);
    }

    pub fn with_state(mut self, state: AgentState) -> Self {
        self.state = state;
        self
    }

    pub fn with_tools<T: Tool + 'static>(mut self, tools: Vec<T>) -> Self {
        for tool in tools {
            self.state.tools.push(tool.define());
            self.executor = self.executor.register(tool);
        }
        self
    }

    /// 运行 Agent，处理 prompts 并返回最终消息列表
    /// 事件通过 broadcaster 实时广播给所有订阅的处理器
    pub async fn run(&mut self, prompts: Vec<Message>) -> Result<Vec<Message>> {
        let abort = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let session_id = "session-1".to_string();
        let message_id = format!("msg-{}", uuid::Uuid::new_v4());

        self.state.messages.extend(prompts.clone());

        // 广播初始事件
        self.broadcast(&AgentEvent::AgentStart);
        self.broadcast(&AgentEvent::TurnStart);

        for prompt in &prompts {
            self.broadcast(&AgentEvent::MessageStart { 
                message: prompt.clone(), 
            });
            self.broadcast(&AgentEvent::MessageEnd { 
                message: prompt.clone(), 
            });
        }

        // 执行主循环
        let result = Self::run_loop(
            std::mem::take(&mut self.state.messages),
            std::mem::take(&mut self.state.tools),
            std::mem::take(&mut self.state.system_prompt),
            self.config.clone(),
            self.executor.clone(),
            session_id,
            message_id,
            abort,
            &self.broadcaster,
        ).await;

        match result {
            Ok(msgs) => {
                self.broadcast(&AgentEvent::AgentEnd { messages: msgs.clone() });
                self.state.messages = msgs.clone();
                Ok(msgs)
            }
            Err(e) => {
                self.broadcast(&AgentEvent::Error { error: e.to_string() });
                Ok(vec![])
            }
        }
    }

    async fn run_loop(
        mut messages: Vec<Message>,
        tools: Vec<super::types::ToolDefine>,
        system_prompt: String,
        config: AgentLoopConfig,
        executor: ToolExecutor,
        session_id: String,
        message_id: String,
        abort: Arc<std::sync::atomic::AtomicBool>,
        broadcaster: &EventBroadcaster,
    ) -> Result<Vec<Message>> {
        loop {
            let system = if system_prompt.is_empty() {
                None
            } else {
                Some(system_prompt.clone())
            };

            let mut context = ai::types::Context::new().with_messages(messages.clone());
            if let Some(prompt) = system {
                context = context.with_system_prompt(prompt);
            }
            if let Some(reasoning) = config.reasoning {
                context = context.with_thinking(reasoning);
            }
            if !tools.is_empty() {
                let ai_tools: Vec<ai::types::Tool> = tools
                    .iter()
                    .map(|t| ai::types::Tool {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        parameters: t.parameters.clone(),
                    })
                    .collect();
                context = context.with_tools(ai_tools);
            }

            let mut stream = client::stream(&config.model, &context).await?;
            let mut partial_message: Option<Message> = None;

            while let Some(result) = stream.next().await {
                if abort.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err(anyhow!("Aborted"));
                }

                let event = result.map_err(|e| anyhow!("Stream error: {}", e))?;

                match event {
                    StreamEvent::Start { .. } => {}
                    StreamEvent::TextStart { .. } => {
                        if partial_message.is_none() {
                            let msg = Message {
                                role: ai::types::Role::Assistant,
                                content: vec![ContentBlock::Text(ai::types::TextContent {
                                    text: String::new(),
                                })],
                                name: None,
                                tool_call_id: None,
                            };
                            partial_message = Some(msg);
                        }
                        let _ = broadcaster.broadcast(&AgentEvent::MessageStart { 
                            message: partial_message.clone().unwrap() 
                        });
                    }
                    StreamEvent::TextDelta { delta, content_index } => {
                        if let Some(ref mut msg) = partial_message {
                            for block in &mut msg.content {
                                if let ContentBlock::Text(t) = block {
                                    t.text.push_str(&delta);
                                }
                            }
                        }
                        let _ = broadcaster.broadcast(&AgentEvent::TextDelta { 
                            content_index, 
                            delta: delta.clone() 
                        });
                    }
                    StreamEvent::TextEnd { content_index, content: _ } => {
                        let _ = broadcaster.broadcast(&AgentEvent::TextEnd { 
                            content_index,
                            content: partial_message.clone().unwrap() 
                        });
                        let _ = broadcaster.broadcast(&AgentEvent::MessageEnd { 
                            message: partial_message.clone().unwrap() 
                        });
                    }
                    StreamEvent::ThinkingStart { content_index } => {
                        if partial_message.is_none() {
                            let msg = Message {
                                role: ai::types::Role::Assistant,
                                content: vec![],
                                name: None,
                                tool_call_id: None,
                            };
                            partial_message = Some(msg);
                        }
                        if let Some(ref mut msg) = partial_message {
                            msg.content.push(ContentBlock::Thinking(ai::types::ThinkingContent {
                                thinking: String::new(),
                                thinking_signature: None,
                                redacted: None,
                            }));
                        }
                        let _ = broadcaster.broadcast(&AgentEvent::ThinkingStart { 
                            content_index 
                        });
                    }
                    StreamEvent::ThinkingDelta { delta, content_index } => {
                        if let Some(ref mut msg) = partial_message {
                            if let Some(ContentBlock::Thinking(t)) = msg.content.last_mut() {
                                t.thinking.push_str(&delta);
                            }
                        }
                        let _ = broadcaster.broadcast(&AgentEvent::ThinkingDelta { 
                            content_index, 
                            delta: delta.clone() 
                        });
                    }
                    StreamEvent::ThinkingEnd { content_index, content: _ } => {
                        let _ = broadcaster.broadcast(&AgentEvent::ThinkingEnd { 
                            content_index,
                            content: partial_message.clone().unwrap()
                        });
                    }
                    StreamEvent::ToolCallStart { content_index } => {
                        if partial_message.is_none() {
                            let msg = Message {
                                role: ai::types::Role::Assistant,
                                content: vec![],
                                name: None,
                                tool_call_id: None,
                            };
                            partial_message = Some(msg);
                        }
                        if let Some(ref mut msg) = partial_message {
                            msg.content.push(ContentBlock::ToolCall(AiToolCall {
                                id: String::new(),
                                name: String::new(),
                                arguments: serde_json::json!({}),
                                reasoning_content: None,
                            }));
                        }
                        let _ = broadcaster.broadcast(&AgentEvent::ToolCallStart { 
                            content_index 
                        });
                    }
                    StreamEvent::ToolCallDelta { delta, content_index } => {
                        if let Some(ref mut msg) = partial_message {
                            for block in &mut msg.content {
                                if let ContentBlock::ToolCall(tc) = block {
                                    if let Some(obj) = tc.arguments.as_object_mut() {
                                        obj.insert(
                                            "_partial".to_string(),
                                            serde_json::Value::String(delta.clone()),
                                        );
                                    }
                                }
                            }
                        }
                        let _ = broadcaster.broadcast(&AgentEvent::ToolCallDelta { 
                            content_index, 
                            delta: delta.clone() 
                        });
                    }
                    StreamEvent::ToolCallEnd { content_index, tool_call } => {
                        if let Some(ref mut msg) = partial_message {
                            if let Some(ContentBlock::ToolCall(tc)) =
                                msg.content.get_mut(content_index)
                            {
                                tc.id = tool_call.id.clone();
                                tc.name = tool_call.name.clone();
                                tc.arguments = tool_call.arguments.clone();
                            }
                        }
                        let agent_tool_call = super::types::ToolCall {
                            id: tool_call.id.clone(),
                            name: tool_call.name.clone(),
                            arguments: tool_call.arguments.clone(),
                        };
                        let _ = broadcaster.broadcast(&AgentEvent::ToolCallEnd { 
                            content_index,
                            tool_call: agent_tool_call,
                        });
                    }
                    StreamEvent::Done { reason: _, message } => {
                        let msg: Message = Message {
                            role: message.role,
                            content: message.content,
                            name: None,
                            tool_call_id: None,
                        };
                        partial_message = Some(msg);
                        break;
                    }
                    StreamEvent::Error { reason: _, error } => {
                        let error_msg = Message {
                            role: ai::types::Role::Assistant,
                            content: vec![ContentBlock::Text(ai::types::TextContent {
                                text: error.clone(),
                            })],
                            name: None,
                            tool_call_id: None,
                        };
                        partial_message = Some(error_msg);
                        break;
                    }
                }
            }

            let message = match partial_message {
                Some(msg) => msg,
                None => {
                    return Err(anyhow!("Stream ended without message"));
                }
            };

            let has_more_tool_calls = message.content.iter()
                .any(|c| matches!(c, ContentBlock::ToolCall(_)));

            if has_more_tool_calls {
                let ctx = ToolContext {
                    session_id: session_id.to_string(),
                    message_id: message_id.to_string(),
                    agent_name: "build".to_string(),
                    abort: abort.clone(),
                };

                let tool_calls: Vec<ToolCall> = message
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        ContentBlock::ToolCall(tc) => Some(ToolCall {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                        }),
                        _ => None,
                    })
                    .collect();

                for tool_call in &tool_calls {
                    let _ = broadcaster.broadcast(&AgentEvent::ToolExecutionStart {
                        tool_call_id: tool_call.id.clone(),
                        tool_name: tool_call.name.clone(),
                        args: tool_call.arguments.clone(),
                    });
                }

                match executor.execute(tool_calls.clone(), ctx).await {
                    Ok(results) => {
                        let mut tool_results = Vec::new();
                        // 关键修复：先 push assistant message，再 push tool_result
                        // 这样消息顺序才是 [user, assistant, tool_result] 而不是 [user, tool_result, assistant]
                        messages.push(message.clone());
                        
                        for (i, result) in results.into_iter().enumerate() {
                            let tool_call = &tool_calls[i];
                            let tool_result_msg = Message {
                                role: ai::types::Role::ToolResult,
                                content: vec![ContentBlock::Text(ai::types::TextContent {
                                    text: result.output.clone(),
                                })],
                                name: Some(tool_call.name.clone()),
                                tool_call_id: Some(tool_call.id.clone()),
                            };
                            messages.push(tool_result_msg.clone());
                            tool_results.push(tool_result_msg);

                            let is_error = result.output.contains("error") || result.output.contains("not found");
                            let _ = broadcaster.broadcast(&AgentEvent::ToolExecutionEnd {
                                tool_call_id: tool_call.id.clone(),
                                tool_name: tool_call.name.clone(),
                                result: serde_json::json!({
                                    "title": result.title,
                                    "output": result.output,
                                }),
                                is_error,
                            });
                        }

                        let _ = broadcaster.broadcast(&AgentEvent::TurnEnd {
                            message: message.clone(),
                            tool_results,
                        });
                    }
                    Err(e) => {
                        let _ = broadcaster.broadcast(&AgentEvent::Error {
                            error: e.to_string(),
                        });
                    }
                }
            } else {
                let _ = broadcaster.broadcast(&AgentEvent::TurnEnd {
                    message: message.clone(),
                    tool_results: vec![],
                });
                return Ok(messages);
            }
        }
    }
}

impl Default for AgentLoop {
    fn default() -> Self {
        Self::new(AgentLoopConfig::new(
            ai::model_db::get_kimi_model("kimi-k2-turbo-preview").unwrap(),
        ))
    }
}
