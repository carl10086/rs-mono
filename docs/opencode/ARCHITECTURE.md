# OpenCode 架构分析

> 详细的 AgentLoop 设计请参考 [AGENT_LOOP.md](./AGENT_LOOP.md)

## 概述

OpenCode 是一个基于 TypeScript 的 AI 编程助手，使用 Bun runtime + Solid.js TUI + Effect 框架。

## 核心技术栈

| 层级 | 技术 |
|------|------|
| Runtime | Bun |
| TUI | Solid.js + opentui |
| 状态管理 | Effect + 异步上下文 |
| 数据库 | SQLite (Drizzle ORM) |
| AI SDK | Vercel AI SDK |
| IPC | RPC (Worker) |

## 宏观架构

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Entry                           │
│                   (yargs command router)                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────────┐  │
│  │  run     │  │   tui    │  │ session  │  │   agent     │  │
│  │  cmd     │  │  thread  │  │   cmd    │  │    cmd      │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──────┬──────┘  │
│       │              │             │                 │        │
│       ▼              ▼             ▼                 ▼        │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │                      SDK Layer                          │  │
│  │              (createOpencodeClient)                     │  │
│  └────────────────────────┬────────────────────────────────┘  │
│                           │                                   │
│                           ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │                     Server                               │  │
│  │                  (HTTP + WebSocket)                      │  │
│  └────────────────────────┬────────────────────────────────┘  │
│                           │                                   │
│  ┌────────────────────────┴────────────────────────────────┐  │
│  │                   Agent + Session                          │  │
│  │                                                        │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │  │
│  │  │  Agent   │  │ Session  │  │ Message  │  │  Tool   │ │  │
│  │  │ Registry │  │ Manager  │  │ Processor│  │ System  │ │  │
│  │  └──────────┘  └──────────┘  └──────────┘  └─────────┘ │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## 核心模块

### 1. Agent 模块 (`src/agent/agent.ts`)

定义内置 Agent 类型：

| Agent | 模式 | 用途 |
|-------|------|------|
| `build` | primary | 默认执行器 |
| `plan` | primary | 计划模式（禁用编辑工具）|
| `general` | subagent | 通用任务执行 |
| `explore` | subagent | 代码探索 |
| `compaction` | primary | 压缩/摘要生成（隐藏）|
| `title` | primary | 标题生成（隐藏）|
| `summary` | primary | 摘要生成（隐藏）|

每个 Agent 包含：
- `name`, `description`
- `mode`: primary | subagent | all
- `permission`: 权限规则集
- `model`: 可选指定模型
- `prompt`: 可选自定义 prompt

### 2. Session 模块 (`src/session/`)

> 详细设计见 [AGENT_LOOP.md](./AGENT_LOOP.md)

| 文件 | 职责 |
|------|------|
| `index.ts` | Session CRUD、会话树管理、fork |
| `prompt.ts` | **主循环入口** SessionPrompt.loop() |
| `processor.ts` | **流处理器** SessionProcessor |
| `llm.ts` | **LLM 调用封装** |
| `message-v2.ts` | 消息结构与流式处理 |
| `compaction.ts` | 上下文压缩 |
| `retry.ts` | 重试逻辑 |
| `schema.ts` | ID 类型定义 (SessionID, MessageID, PartID) |
| `session.sql.ts` | Drizzle 表定义 |

**Session 数据模型**：
```typescript
interface Session {
  id: SessionID
  slug: string
  projectID: ProjectID
  workspaceID?: WorkspaceID
  directory: string
  parentID?: SessionID  // 用于会话分叉
  title: string
  version: string
  summary?: { additions, deletions, files, diffs }
  share?: { url }
  time: { created, updated, compacting?, archived? }
  permission?: Permission.Ruleset
}
```

### 3. 工具系统 (`src/tool/`)

> 详细设计见 [AGENT_LOOP.md](./AGENT_LOOP.md)

| 工具 | 文件 | 功能 |
|------|------|------|
| glob | `glob.ts` | 文件模式匹配 |
| grep | `grep.ts` | 内容搜索 |
| list | `ls.ts` | 目录列表 |
| read | `read.ts` | 读取文件 |
| write | `write.ts` | 写入文件 |
| edit | `edit.ts` | 编辑文件（diff）|
| bash | `bash.ts` | 执行命令 |
| webfetch | `webfetch.ts` | 获取网页 |
| websearch | `websearch.ts` | 网络搜索 |
| codesearch | `codesearch.ts` | 代码搜索 |
| task | `task.ts` | 启动子 Agent |
| skill | `skill.ts` | 调用 Skill |
| todo | `todo.ts` | TodoWrite 工具 |
| truncate | `truncate.ts` | 截断工具 |

### 4. TUI 模块 (`src/cli/cmd/tui/`)

```
tui/
├── app.tsx          # 主应用入口，Provider 组合
├── thread.ts        # CLI 入口，启动 Worker
├── worker.ts        # RPC Worker 实现
├── routes/
│   ├── home.tsx     # 首页路由
│   └── session/     # Session 相关路由
├── component/       # UI 组件
│   ├── prompt/      # Prompt 输入相关
│   └── dialog-*     # 各种对话框
├── context/         # Solid.js Context providers
│   ├── sdk.tsx      # SDK provider
│   ├── sync.tsx     # 状态同步
│   ├── theme.tsx    # 主题管理
│   └── ...
└── ui/              # 基础 UI 组件
```

**TUI 技术特点**：
- 使用 `opentui/solid` 渲染引擎
- Provider 模式管理状态
- 命令面板 (`CommandProvider`)
- 对话框系统 (Dialog)
- 路由系统 (RouteProvider)

### 5. Provider 系统 (`src/provider/`)

管理 AI 模型提供商：
- Anthropic
- OpenAI
- Google Vertex
- OpenRouter
- 等

## 关键设计决策

### 1. Worker 架构 (IPC)

TUI 使用 Worker 进行进程隔离：
- 主线程：Terminal I/O + React 渲染
- Worker 线程：Agent 执行 + SDK 通信

### 2. Event Bus 系统

使用自定义 Event Bus 进行组件通信：
```typescript
Bus.publish(Event.Updated, { info })
Bus.subscribe(Event.Updated, handler)
```

### 3. 数据库 Schema 命名

使用 snake_case 命名数据库字段：
```typescript
const table = sqliteTable("session", {
  id: text().primaryKey(),
  project_id: text().notNull(),
  created_at: integer().notNull(),
})
```

### 4. Effect 框架

使用 Effect 进行 Effectful 编程：
- `Effect.gen` 用于生成器组合
- `Effect.fn` 用于命名/追踪
- `Effect.Callback` 处理回调

### 5. 权限系统

基于规则集的权限系统：
```typescript
{
  permission: "bash",
  action: "allow" | "deny" | "ask",
  pattern: "*" | "*.py" | ...
}
```

## 依赖关系图

```
agent/agent.ts
    │
    ├── provider/provider.ts
    │       ├── config/config.ts
    │       └── provider/schema.ts
    │
    ├── session/
    │       ├── index.ts
    │       ├── prompt.ts      ← 主循环
    │       ├── processor.ts   ← 流处理
    │       ├── llm.ts         ← LLM 调用
    │       ├── message-v2.ts
    │       └── compaction.ts
    │
    └── permission/permission.ts

cli/cmd/run.ts
    │
    └── createOpencodeClient (SDK)
            │
            └── Server (HTTP/WS)
                    │
                    ├── Agent
                    ├── Session
                    ├── Tool
                    └── Provider
```

## 文件统计

| 模块 | 主要文件数 | 估算行数 |
|------|-----------|----------|
| agent | ~3 | ~500 |
| session | ~15 | ~3000 |
| tool | ~20 | ~4000 |
| provider | ~10 | ~1500 |
| cli/tui | ~50+ | ~10000+ |
| **总计** | **~100** | **~19000+** |

## 参考价值

1. **Agent 设计**: 内置多种 Agent 类型，权限隔离
2. **Session 管理**: 会话树、fork、消息流式处理
3. **TUI 架构**: Provider 模式、命令面板、路由系统
4. **工具系统**: 工具注册、权限控制、元数据
5. **压缩机制**: 基于 token 阈值的自动压缩
