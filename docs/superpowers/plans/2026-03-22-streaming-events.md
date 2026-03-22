# 流式事件系统实现计划

> **Goal:** 参考 pi-mono 的 EventStream 设计，为 AgentLoop 实现真正的流式事件系统

**Architecture:** 
- 使用 `mpsc::channel` + `futures::Stream` 实现事件推送
- `AgentLoop::run_stream()` 返回 `(impl Stream<Item=AgentEvent>, oneshot::Receiver<Vec<Message>>)`
- Tokio runtime 处理背压（mpsc 天然支持）

**Tech Stack:** 
- `futures` crate: Stream trait, StreamExt
- `tokio` sync: mpsc, oneshot
- 现有 `tracing` 用于调试日志

---

## 文件变更概览

| 文件 | 操作 |
|------|------|
| `apps/coding-agent/src/agent/event_stream.rs` | 新建 - EventStream 封装 |
| `apps/coding-agent/src/agent/types.rs` | 修改 - 细化 AgentEvent |
| `apps/coding-agent/src/agent/agent_loop.rs` | 修改 - 新增 run_stream() |
| `apps/coding-agent/examples/01_basic.rs` | 修改 - 使用流式消费 |

---

## 实现步骤

### Task 1: 创建 EventStream 封装

**Files:**
- Create: `apps/coding-agent/src/agent/event_stream.rs`

**实现:**

```rust
use futures::Stream;
use tokio::sync::mpsc;

pub struct EventStream<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T: Send + 'static> EventStream<T> {
    pub fn new() -> (Self, mpsc::Sender<T>) {
        let (tx, rx) = mpsc::channel(100);
        (EventStream { receiver: rx }, tx)
    }
}

impl<T: Send + 'static> Stream for EventStream<T> {
    type Item = T;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.receiver.poll_recv(cx).map(|opt| opt.map(Ok))
    }
}
```

**验证:**
```bash
cargo build -p coding-agent
```

---

### Task 2: 细化 AgentEvent 类型

**Files:**
- Modify: `apps/coding-agent/src/agent/types.rs:48-83`

**修改 AgentEvent:**

```rust
pub enum AgentEvent {
    // 生命周期
    AgentStart,
    AgentEnd { messages: Vec<Message> },
    TurnStart,
    TurnEnd { message: Message, tool_results: Vec<Message> },
    
    // 消息流式更新 - 细粒度
    MessageStart { message: Message },
    TextDelta { content_index: usize, delta: String },
    ThinkingDelta { content_index: usize, delta: String },
    ToolCallDelta { content_index: usize, delta: String },
    MessageEnd { message: Message },
    
    // 工具执行
    ToolExecutionStart { tool_call_id: String, tool_name: String, args: Value },
    ToolExecutionUpdate { tool_call_id: String, partial_result: String },
    ToolExecutionEnd { tool_call_id: String, tool_name: String, result: Value, is_error: bool },
    
    Error { error: String },
}
```

**验证:**
```bash
cargo build -p coding-agent 2>&1 | grep -E "(error|warning)"
```

---

### Task 3: 实现 AgentLoop::run_stream()

**Files:**
- Modify: `apps/coding-agent/src/agent/agent_loop.rs`

**关键设计:**

1. **新增 run_stream 方法**:
```rust
pub async fn run_stream(&mut self, prompts: Vec<Message>) 
    -> Result<(impl Stream<Item=AgentEvent> + Send, oneshot::Receiver<Vec<Message>>)>
```

2. **内部改造**:
- 创建 `(EventStream<AgentEvent>, tx)`
- AI 流式响应时：`tx.send(AgentEvent::TextDelta { ... }).await?`
- 工具执行时：`tx.send(AgentEvent::ToolExecutionStart { ... }).await?`
- 结束时：`tx.send(AgentEvent::AgentEnd { messages: ... }).await?` + `result_tx.send(messages)`

3. **保留旧 run() 方法**:
```rust
pub async fn run(&mut self, prompts: Vec<Message>) -> Result<Vec<AgentEvent>> {
    let (events, result) = self.run_stream(prompts).await?;
    futures::pin_mut!(events);
    let mut collected = Vec::new();
    while let Some(event) = events.next().await {
        collected.push(event);
    }
    result.await?;
    Ok(collected)
}
```

**验证:**
```bash
cargo build -p coding-agent
```

---

### Task 4: 更新 example 使用流式消费

**Files:**
- Modify: `apps/coding-agent/examples/01_basic.rs`

**新消费方式:**

```rust
let (mut events, result) = agent.run_stream(prompts).await?;
futures::pin_mut!(events);

while let Some(event) = events.next().await {
    match &event {
        AgentEvent::TextDelta { delta, .. } => print!("{}", delta),
        AgentEvent::ToolExecutionStart { tool_name, args, .. } => {
            print!("{}  ├─ call {}{}", DIM, tool_name, RESET);
            if let Some(obj) = args.as_object() {
                for (k, v) in obj.iter().filter(|(k, _)| *k != "_partial") {
                    print!("({}={})", k, v);
                }
            }
            println!();
        }
        AgentEvent::ToolExecutionEnd { is_error, .. } => {
            if *is_error {
                println!("{}  └─ error{}", DIM, RED);
            } else {
                println!("{}  └─ done{}", DIM, GREEN);
            }
        }
        _ => {}
    }
}
result.await?;
println!("\n{}Done.{}", BOLD, RESET);
```

**验证:**
```bash
KIMI_API_KEY=$KIMI_API_KEY cargo run -p coding-agent --example 01_basic
```

---

## 风险点

1. **背压**: FIXME - LLM 生成速度一般比 UI 慢，概率不大
2. **mpsc 容量**: 100 缓冲是否足够？可动态调整
3. **事件丢失**: 如果消费者崩溃，tx.send 会返回 Err，可记录日志

---

## 验证命令

```bash
# 构建验证
cargo build -p coding-agent

# 运行 example
KIMI_API_KEY=$KIMI_API_KEY cargo run -p coding-agent --example 01_basic

# 检查警告
cargo build -p coding-agent 2>&1 | grep -E "warning"
```
