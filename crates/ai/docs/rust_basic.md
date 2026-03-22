# Rust 入门：对比 Python 理解

## 1. 模块系统

```rust
pub mod client;           // 声明模块
mod providers;            // 私有模块

pub use client::Client;   // 重新导出
```

**Python:**
```python
from client import Client
```

---

## 2. 结构体 → dataclass

```rust
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,  // Vec<T> 类似 List[T]
    pub name: Option<String>,      // Option<T> 类似 Optional[T]
}
```

**Python:**
```python
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Message:
    role: Role
    content: List[ContentBlock]
    name: Optional[str] = None
```

---

## 3. 枚举

### 普通枚举

```rust
pub enum Role {
    User,
    Assistant,
    System,
}
```

**Python:**
```python
from enum import Enum

class Role(Enum):
    USER = "user"
    ASSISTANT = "assistant"
    SYSTEM = "system"
```

### 带数据的枚举

```rust
pub enum ContentBlock {
    Text(TextContent),
    Thinking(ThinkingContent),
    ToolCall(ToolCall),
}
```

**Python:** 用 Union 模拟，无法强制穷举

---

## 4. Trait → Protocol

```rust
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    async fn stream(&self, context: Context) -> Result<StreamResponse>;
}
```

**Python:**
```python
from typing import Protocol

class Provider(Protocol):
    @property
    def name(self) -> str: ...
    async def stream(self, context) -> StreamResponse: ...
```

---

## 5. Option → Optional

```rust
pub name: Option<String>,

if let Some(name) = &self.name { ... }
```

**Python:**
```python
name: str | None = None

if name is not None:
    ...
```

---

## 6. async/await

Rust 和 Python 语法几乎一样，但运行时不同。

```rust
async fn stream(&self) -> Result<StreamResponse> {
    let response = self.client.post().send().await?;
    Ok(StreamResponse::new(response))
}
```

**Python:**
```python
async def stream():
    response = await client.post().send()
    return StreamResponse(response)
```

---

## 7. 完整对照表

| Rust | Python |
|------|--------|
| `struct` | `@dataclass` |
| `enum` + 数据 | Union type |
| `trait` | `Protocol` |
| `Option<T>` | `T \| None` |
| `async fn` | `async def` |
| `?` 运算符 | 无 |
| `match` | if/elif |

---

# 以下是 Rust 独有的概念（需要专门学习）

---

# 核心概念 1：借用（& 和 &mut）

## 这是 Rust 和 Python 最大的区别

Python 里，所有赋值都是"传值"或"传引用"，你不需要操心。

但 Rust 要**手动管理内存**，通过"借用"来控制谁可以使用数据。

## 借用规则

```rust
fn main() {
    let s = String::from("hello");  // s 拥有这个 String

    // &s：借用 s，不获取所有权（只读）
    let r1 = &s;
    let r2 = &s;                    // 可以有多个不可变借用
    println!("{} and {}", r1, r2);
    // r1, r2 在这里不再使用

    // &mut s：可变借用（同一时刻只能有一个！）
    let r3 = &mut s;
    r3.push_str(", world");
    println!("{}", r3);
}
```

## 对比 Python

```python
s = "hello"

# Python 没有借用概念，赋值就是引用或拷贝
r1 = s
r2 = s
print(f"{r1} and {r2}")

# Python 没有 &mut 等价物
s = s + ", world"  # 创建了新字符串
print(s)
```

**Rust 的规则（编译器强制）：**
1. 可以有**多个不可变引用** `&T`
2. 只能有**一个可变引用** `&mut T`
3. 不可变引用和可变引用**不能同时存在**

## 为什么要有借用？

**目的：防止数据竞争（data race）**

比如：
- 两个线程同时修改同一个数据 → 程序崩溃
- 你还在用数据，另一个把它改了 → 潜在 bug

Rust 编译器在**编译时**就禁止这些情况，而不是等到运行时才报错。

## 在 struct 方法中使用借用

```rust
impl Message {
    // &self：只读借用（类似 Python 的 self）
    pub fn text_content(&self) -> Option<&str> {
        for block in &self.content {
            if let ContentBlock::Text(t) = block {
                return Some(&t.text);
            }
        }
        None
    }

    // mut self：可变借用
    pub fn with_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }
}
```

**Python 等价：**
```python
class Message:
    def text_content(self):  # Python 没有借用概念
        for block in self.content:
            if isinstance(block, TextContent):
                return block.text
        return None

    def with_message(self, message):  # Python 总是可以修改
        self.messages.append(message)
        return self
```

---

# 核心概念 2：match 穷举检查

## Rust 的 match

```rust
match event {
    StreamEvent::Start { partial } => {
        println!("开始: {:?}", partial);
    }
    StreamEvent::Done { reason, message } => {
        println!("完成: {:?}", reason);
    }
    StreamEvent::Error { reason, error } => {
        println!("错误: {}", error);
    }
    // 漏掉任何一个变体？编译报错！
}
```

**关键：漏掉任何一种情况，编译器直接报错。**

## Python 没有这个

```python
# Python 靠 if/elif，漏了也不会报错
if isinstance(event, Start):
    print("开始")
elif isinstance(event, Done):
    print("完成")
# 漏了 Error？运行时才发现
```

## Kotlin 的 when 类似

```kotlin
when (event) {
    is Start -> { println("开始") }
    is Done -> { println("完成") }
    is Error -> { println("错误") }
    // 漏了？编译报错
}
```

---

# 核心概念 3：? 运算符

## Rust 的 ?

```rust
async fn stream(&self) -> Result<StreamResponse> {
    // ? 的作用：如果 Result 是 Err，立即返回这个错误
    let response = req_builder.send().await?;
    let status = response.status();
    Ok(StreamResponse::new(response))
}
```

等价于：

```rust
let response = match req_builder.send().await {
    Ok(r) => r,
    Err(e) => return Err(e.into()),
};
```

## Python 没有 ?

```python
async def stream():
    try:
        response = await req_builder.send()
        return StreamResponse(response)
    except Exception as e:
        raise  # 手动传播
```

---

# 核心概念 4：Send + Sync（线程安全标记）

## Rust 的 trait 约束

```rust
pub trait Provider: Send + Sync {
    // ...
}
```

- `Send`：可以安全地在线程间传递
- `Sync`：可以安全地跨线程共享引用

**编译器检查：** 如果你的类型不满足这些约束却跨了线程，编译报错。

## Python/Kotlin 没有

Python 和 Kotlin 的线程安全由运行时检查（或者根本不管）。

```python
# Python：线程安全全靠程序员自觉
import threading

# 可能崩溃，可能没事，全看运气
shared = []

def worker():
    shared.append(1)

threading.Thread(target=worker).start()
```

---

# 完整例子对比

## Python 版本

```python
from dataclasses import dataclass
from typing import List

@dataclass
class Message:
    role: str
    content: List[str]

def create_message(content: str) -> Message:
    return Message(role="user", content=[content])

msg = create_message("Hello")
print(msg)
```

## Rust 版本

```rust
#[derive(Debug)]
pub struct Message {
    pub role: String,
    pub content: Vec<String>,
}

impl Message {
    // self 是不可变借用
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: vec![content.into()],
        }
    }
}

fn main() {
    let msg = Message::user("Hello");
    println!("{:?}", msg);
}
```

---

# 学习路径建议

1. **先忽略生命周期** - 大部分情况编译器自动推断
2. **先学会用 `&` 和 `&mut`** - 这是 Rust 最核心的概念
3. **习惯用 `match`** - 比 if/elif 更安全
4. **习惯用 `?`** - 让错误处理更简洁
5. **理解 `Option` 和 `Result`** - 替代异常做错误处理

## 快速上手口诀

- **看到 `&` 就是"借用"，不获取所有权**
- **看到 `?` 就是"可能失败"，失败就提前返回**
- **看到 `match` 就要处理所有情况**
- **看到 `Vec<String>` 就是 `List[str]`**
- **看到 `Option<T>` 就是 `T | None`**
