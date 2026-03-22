# Rs-Mono

## 目的

实现一个 **coding-agent**，使用 Rust workspace 设计，参考 [pi-mono](refer/pi-mono) 和 [opencode](refer/opencode) 的优点。

## 项目结构

```
rs-mono/
├── refer/          # 参考项目（不参与构建）
│   ├── pi-mono/    # TS agent 库，架构参考
│   └── opencode/   # 已实现的 TS coding-agent
│
├── libs/           # 核心库
│   ├── ai/         # LLM 调用封装
│   └── agent/      # Agent loop 实现
│
└── apps/           # 应用
    └── coding/     # Coding agent 主程序
```

## 技术选型

- **多 workspace**：Rust workspace 模式
- **架构参考**：pi-mono 的模块化设计
- **核心模块**：
  - `ai` - 模型调用（OpenAI/Claude 等）
  - `agent` - Agent 循环、工具调用、状态管理

## 沟通语言

- 所有沟通必须使用中文
