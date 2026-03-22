# Coding Agent 源码解读

## 1. 项目概览

### 1.1 项目结构

```mermaid
graph TD
    RS["rs-mono (Workspace)"]
    RS --> CRATES["crates/ai (AI模块)"]
    RS --> APPS["apps/coding-agent (应用)"]
    
    CRATES --> AI_LIB["ai/src/lib.rs"]
    CRATES --> AI_TYPES["ai/src/types.rs"]
    CRATES --> AI_PROVIDER["ai/src/provider.rs"]
    CRATES --> AI_CLIENT["ai/src/client.rs"]
    CRATES --> AI_KIMI["ai/src/providers/kimi.rs"]
    
    APPS --> AGENT["agent/"]
    APPS --> TOOLS["tools/"]
    APPS --> MAIN["main.rs"]
    
    AGENT --> AGENT_LOOP["agent_loop.rs"]
    AGENT --> EXECUTOR["executor.rs"]
    AGENT --> TYPES["types.rs"]
    TOOLS --> READ["read.rs"]
```

### 1.2 核心模块职责

| 模块 | 路径 | 职责 |
|------|------|------|
| `ai` | `crates/ai/` | LLM 调用封装：Provider 接口、模型配置、流式响应 |
| `agent` | `apps/coding-agent/src/agent/` | Agent 循环：消息处理、工具调用、状态管理 |
| `tools` | `apps/coding-agent/src/tools/` | 工具实现：ReadTool 等具体工具 |

---

## 2. 数据流架构

```mermaid
sequenceDiagram
    participant User as 用户
    participant Agent as AgentLoop
    participant Client as ai::client
    participant Provider as KimiProvider
    participant Tool as ReadTool
    
    User->>Agent: run(prompts)
    
    rect rgb(200, 220, 240)
        Note over Agent,Provider: LLM 对话循环
        loop 直到没有工具调用
            Agent->>Client: stream(model, context)
            Client->>Provider: stream(context)
            Provider-->>Client: StreamResponse
            Client-->>Agent: StreamEvent 流
            
            alt 有工具调用
                Agent->>Tool: execute(args)
                Tool-->>Agent: ToolResult
                Agent->>Agent: 循环继续
            else 没有工具调用
                Note over Agent: 结束
            end
        end
    end
    
    Agent-->>User: Vec<AgentEvent>
```

---

## 3. 核心类型详解

### 3.1 消息类型 (types.rs)

```mermaid
classDiagram
    class Message {
        +Role role
        +Vec~ContentBlock~ content
        +Option~String~ name
        +Option~String~ tool_call_id
        +user(content) Message
        +assistant(content) Message
        +tool_result() Message
        +system(content) Message
    }
    
    class ContentBlock {
        <<enumeration>>
        Text(TextContent)
        Thinking(ThinkingContent)
        Image(ImageContent)
        ToolCall(ToolCall)
    }
    
    class Role {
        <<enumeration>>
        User
        Assistant
        ToolResult
        System
    }
    
    Message --> ContentBlock
    ContentBlock --> Role
```

**示例代码**：

```rust
// 用户消息
let user_msg = Message::user(vec![ContentBlock::text("List files")]);

// Assistant 响应
let assistant_msg = Message::assistant(vec![
    ContentBlock::text("Here are the files:"),
    ContentBlock::tool_call("call_1", "read", json!({"path": "/tmp"}))
]);

// 工具结果
let tool_result = Message::tool_result("call_1", "read", vec![
    ContentBlock::text("file1.rs\nfile2.rs")
]);
```

### 3.2 工具类型 (executor.rs)

```mermaid
classDiagram
    class ToolExecutor {
        +HashMap~String, Arc~dyn Tool~~ tools
        +register(tool) ToolExecutor
        +execute(tool_calls, ctx) Vec~ToolResult~
    }
    
    class Tool {
        <<interface>>
        +define() ToolDefine
        +execute(args, ctx) Pin~Future~
    }
    
    class ToolDefine {
        +String name
        +String description
        +serde_json::Value parameters
    }
    
    class ToolCall {
        +String id
        +String name
        +serde_json::Value arguments
    }
    
    class ToolResult {
        +String title
        +String output
        +serde_json::Value metadata
    }
    
    ToolExecutor o-- Tool
    Tool ..> ToolDefine
    ToolExecutor ..> ToolCall
    ToolExecutor ..> ToolResult
```

**示例**：如何注册一个工具

```rust
// 1. 定义工具
pub struct ReadTool;

impl Tool for ReadTool {
    fn define(&self) -> ToolDefine {
        ToolDefine {
            name: "read".into(),
            description: "Read file contents".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "filePath": { "type": "string" }
                }
            }),
        }
    }
    
    fn execute(&self, args: Value, ctx: ToolContext) -> Pin<Box<dyn Future<Output=Result<ToolResult>> + Send>> {
        Box::pin(async move {
            let path = args["filePath"].as_str().unwrap();
            let content = tokio::fs::read_to_string(path).await?;
            Ok(ToolResult::new("read", content))
        })
    }
}

// 2. 注册到 Agent
let agent = AgentLoop::new(config)
    .with_tools(vec![ReadTool::new()]);
```

---

## 4. Provider 接口设计

### 4.1 抽象接口 (provider.rs)

```mermaid
classDiagram
    class Provider {
        <<interface>>
        +name() &str
        +api() Api
        +model_id() &str
        +stream(context) Result~StreamResponse~
        +complete(context) Result~AssistantMessage~
    }
    
    class KimiProvider {
        +Client client
        +String model
        +String api_key
        +stream(context) Result~StreamResponse~
    }
    
    class Context {
        +Option~String~ system_prompt
        +Vec~Message~ messages
        +Vec~Tool~ tools
        +Option~ThinkingLevel~ thinking
    }
    
    Provider <|.. KimiProvider
    KimiProvider --> Context
```

### 4.2 流式事件 (stream_event.rs)

```mermaid
stateDiagram-v2
    [*] --> Start
    Start --> TextStart: content_block_start (text)
    Start --> ToolCallStart: content_block_start (tool_use)
    Start --> ThinkingStart: content_block_start (thinking)
    
    TextStart --> TextDelta: text_delta
    TextDelta --> TextDelta: text_delta
    TextDelta --> TextEnd: content_block_stop
    
    ToolCallStart --> ToolCallDelta: input_json_delta
    ToolCallDelta --> ToolCallEnd: content_block_stop
    
    ThinkingStart --> ThinkingDelta: thinking_delta
    ThinkingDelta --> ThinkingEnd: content_block_stop
    
    TextEnd --> Done: message_stop
    ToolCallEnd --> Done: message_stop
    ThinkingEnd --> Done: message_stop
    
    Done --> [*]
    
    Note: 任意状态 --> Error: error event
    Error --> [*]
```

---

## 5. Agent Loop 执行流程

### 5.1 主循环 (agent_loop.rs)

```mermaid
flowchart TD
    START([开始]) --> INIT[初始化 Context]
    INIT --> STREAM[调用 client::stream]
    STREAM --> LOOP{处理流事件}
    
    LOOP -->|TextDelta| APPEND_TEXT[追加文本到 Message]
    LOOP -->|ThinkingDelta| APPEND_THINKING[追加思考内容]
    LOOP -->|ToolCallDelta| APPEND_TOOL[追加工具参数]
    LOOP -->|Done| CHECK{检查 stop_reason}
    
    APPEND_TEXT --> CONTINUE
    APPEND_THINKING --> CONTINUE
    APPEND_TOOL --> CONTINUE
    
    CONTINUE[继续接收事件] --> LOOP
    
    CHECK -->|Stop| EXTRACT[提取完整 Message]
    CHECK -->|ToolUse| EXEC[执行工具调用]
    
    EXEC --> ADD_RESULT[添加工具结果到 messages]
    ADD_RESULT --> STREAM
    
    EXTRACT --> RETURN([返回结果])
    
    CHECK -->|Error/Aborted| ERROR[发送 Error 事件]
    ERROR --> RETURN
```

### 5.2 工具执行流程

```mermaid
sequenceDiagram
    participant AI as AI (LLM)
    participant AL as AgentLoop
    participant TE as ToolExecutor
    participant RT as ReadTool
    
    AI->>AL: "read file: /path/to/file"
    AL->>TE: execute([ToolCall])
    TE->>RT: execute(args, ctx)
    
    rect rgb(240, 248, 255)
        Note over RT: ReadTool::execute 内部
        RT->>RT: 解析参数
        alt 是目录
            RT->>RT: tokio::fs::read_dir
        else 是文件
            RT->>RT: tokio::fs::read_to_string
        end
        RT->>RT: 格式化输出
        RT->>RT: 返回 ToolResult
    end
    
    RT-->>TE: ToolResult
    TE-->>AL: Vec~ToolResult~
    AL->>AL: 转换为 Message::tool_result
    AL->>AI: 继续下一轮对话
```

---

## 6. 关键代码解析

### 6.1 Client 注册机制 (client.rs)

```rust
// 使用 OnceLock 实现全局单例
static REGISTRY: OnceLock<Mutex<ApiProviderRegistry>> = OnceLock::new();

pub fn register_provider<P: Provider + 'static>(provider: P) {
    // 获取或创建注册表
    get_registry().lock().unwrap().register(provider);
}

pub async fn stream(model: &Model, context: &Context) -> Result<StreamResponse> {
    let registry = get_registry().lock().unwrap();
    // 根据 Model.api 查找对应的 Provider
    let provider = registry.get(&model.api)?;
    provider.stream(context).await
}
```

**设计思想**：这种模式允许运行时动态注册不同的 Provider，实现**插件式**的 Provider 加载。

### 6.2 ToolExecutor 的注册模式 (executor.rs)

```rust
#[derive(Clone)]
pub struct ToolExecutor {
    tools: Arc<HashMap<String, Arc<dyn Tool>>>,  // 使用 Arc 支持克隆
}

impl ToolExecutor {
    // 返回新的实例，实现不可变注册
    pub fn register<T: Tool + 'static>(self, tool: T) -> Self {
        let mut new_tools = (*self.tools).clone();
        new_tools.insert(tool.define().name, Arc::new(tool));
        Self {
            tools: Arc::new(new_tools),
        }
    }
}
```

**设计思想**：使用 `Arc<HashMap<...>>` 让 `ToolExecutor` 可以 `Clone`，同时保持内部状态共享。

### 6.3 SSE 流式解析 (kimi.rs)

```rust
struct SseStream<S> {
    inner: S,                    // 原始字节流
    buffer: Bytes,               // 缓冲区
    current_event: Option<String>,
    current_data: String,
}

// 实现 Stream trait，手动解析 SSE 格式
impl<S: Stream> Stream for SseStream<S> {
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // 1. 从缓冲区查找行结束符
            if let Some(pos) = find_line_end(&self.buffer) {
                let line = decode_line(&self.buffer[..pos]);
                self.buffer.advance(pos + 1);
                
                // 2. 解析 SSE 字段
                if let Some(colon_pos) = line.find(':') {
                    match &line[..colon_pos] {
                        "event" => self.current_event = Some(value),
                        "data" => self.current_data.push_str(&value),
                        _ => {}
                    }
                }
                
                // 3. 空行表示一个事件结束
                if line.is_empty() && (self.current_data.is_empty() || self.current_event.is_some()) {
                    return Poll::Ready(Some(Ok(SseEvent { ... })));
                }
            }
            // ...
        }
    }
}
```

**SSE 格式示例**：
```
event: message
data: {"type": "content_block_start", "index": 0}

event: ping
data: 

event: message
data: {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}
```

---

## 7. 入口程序解析 (main.rs)

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化 tracing 日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. 创建 Kimi Provider 并注册
    let api_key = std::env::var("KIMI_API_KEY")?;
    let provider = KimiProvider::new("kimi-k2-turbo-preview", api_key);
    register_provider(provider);

    // 3. 获取模型配置
    let model = model_db::get_kimi_model("kimi-k2-turbo-preview")?;

    // 4. 创建 Agent 并注册工具
    let mut agent = AgentLoop::new(AgentLoopConfig::new(model))
        .with_tools(vec![ReadTool::new()]);

    // 5. 构建用户消息
    let prompts = vec![Message::user(vec![ContentBlock::text(
        "List the files in the current directory using the read tool"
    )])];

    // 6. 运行并获取事件流
    match agent.run(prompts).await {
        Ok(events) => {
            for event in &events {
                println!("{:?}", event);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
    Ok(())
}
```

---

## 8. 理解要点

### 8.1 为什么这样设计？

| 设计模式 | 好处 | 示例 |
|----------|------|------|
| **Trait Object (Provider)** | 支持多种 AI 提供商动态替换 | `KimiProvider`, 未来可加 `OpenAIProvider` |
| **Arc + Clone** | ToolExecutor 可自由克隆，状态共享 | `tools: Arc<HashMap<...>>` |
| **Stream abstraction** | 统一处理 SSE、websocket 等多种流 | `StreamResponse` 封装 |
| **OnceLock singleton** | 全局唯一注册表，线程安全 | `REGISTRY: OnceLock<...>` |
| **Builder pattern** | 流畅配置 Agent | `.with_tools().with_reasoning()` |

### 8.2 调试建议

查看流事件：
```rust
// 在 kimi.rs 的 stream 方法中
debug!(data = %data, "SSE");

// 在 client.rs 中
let provider = registry.get(&model.api)?;
// 添加日志
tracing::debug!("Calling provider: {}", provider.name());
```

---

## 9. 下一步探索

1. **扩展 Provider**：参考 `kimi.rs` 实现 `OpenAIProvider` 或 `ClaudeProvider`
2. **添加工具**：在 `tools/` 目录下实现 `BashTool`, `WriteTool` 等
3. **状态持久化**：为 `AgentState` 添加存储机制
4. **流式输出**：将 `AgentEvent` 流式传输到 UI

---

## 10. 快速问答

**Q: 为什么用 `Pin<Box<dyn Future>>`？**
A: 因为 `Tool::execute` 返回的 Future 可能包含引用，需要固定内存位置。

**Q: `AgentLoop::run` vs `run_stream` 区别？**
A: `run` 返回所有事件组成的 Vec，`run_stream` 返回实时 Stream。

**Q: 如何添加新的 AI 提供商？**
A: 实现 `Provider` trait → 在 `providers/` 创建 `xxx.rs` → 在 `main.rs` 注册。
