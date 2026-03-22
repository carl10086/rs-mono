---
name: pi-rust-diff
description: |
  对比 pi-mono (TypeScript) 和 rs-mono (Rust) 项目 ai 模块的功能实现差异。
  通过阅读 pi-mono 源码学习优秀设计，优化 rs-mono 的 ai 模块实现。
  参考 opencode 的架构来设计 rs-mono。
  当用户提到"对比 pi-mono 和 rust"、"检查模块差异"、"rust 实现对标 pi-mono"时触发。
---

# Pi-Mono vs Rs-Mono AI 模块对比分析器

通过阅读 pi-mono (TypeScript) 源码学习优秀设计，优化 rs-mono (Rust) 的 `ai` 模块实现。
架构参考 opencode，功能学习 pi-mono。

## 当前项目状态

- **rs-mono**：仅保留 `ai` crate (`crates/ai/`)
- **agent 模块**：已删除，未来基于 opencode 架构重新设计
- **目标**：参考 opencode 的架构来设计 rs-mono

## 工作流程

### 1. 确定对比范围

**rs-mono (当前项目)**：
- `crates/ai/src/` - 主要源代码
- `crates/ai/examples/` - 示例代码

**pi-mono (参考学习)**：
- `refer/pi-mono/packages/ai/src/` - TypeScript 源代码
- 用于学习优秀设计，不是照搬

**opencode (架构参考)**：
- `refer/opencode/` - TypeScript coding-agent 实现
- 用于参考架构设计

### 2. 分析维度

#### A. 文件结构对比
- 列出两边的文件清单
- 识别缺失的文件
- 对比目录结构差异

#### B. 核心类型/接口对比
- Rust struct/enum vs TypeScript interface/type
- 字段名称和类型映射
- 可选/必填字段差异

#### C. 核心功能对比
- 结构体和方法的存在性
- 方法签名对比
- 功能实现完整性

#### D. 架构模式对比
- 设计模式一致性
- 调用流程对比
- 事件/生命周期管理

### 3. 生成对比报告

#### 输出格式

**第一步：输出功能点对比总览表格**

```markdown
## 📊 功能点对比总览

| 功能点 | pi-mono | rs-mono | 状态 |
|--------|---------|---------|------|
| {功能1} | ✅ | ✅ | 完整 |
| {功能2} | ✅ | ❌ | 缺失 |
| {功能3} | ✅ | ⚠️ | 部分 |
```

状态说明：
- ✅ 完整：两边都有，功能等价
- ⚠️ 部分：一方有但实现不完整
- ❌ 缺失：一方的功能在另一方完全不存在

**第二步：输出关键缺失（阻断功能）**

按优先级 P0 > P1 > P2 排列，每个缺失包含：

```markdown
### N. {缺失功能名称}
**pi-mono 实现**:
```typescript
// 代码片段或描述
```

**rs-mono 现状**:
- 状态：完全缺失 / 部分实现
- 影响：{说明此功能缺失的后果}

**建议优先级**：P0 / P1 / P2
```

**第三步：输出次要差异**

```markdown
## 🟡 次要差异

1. **{差异点}**
   - pi-mono: {描述}
   - rs-mono: {描述}
   - 影响：{说明}
```

**第四步：输出优先级建议表格**

```markdown
## 💡 建议的优先级

| 优先级 | 任务 | 工作量 | 影响 |
|--------|------|--------|------|
| P0 | {任务} | {大小} | {说明} |
| P1 | {任务} | {大小} | {说明} |
```

**第五步：输出待确认问题**

```markdown
## ❓ 我的问题

1. 是否优先实现 **{最高优先级任务}**？
2. {其他关键问题}
```

### 4. 实现流程（针对每个差异点）

**Step 1: 阅读 pi-mono 源码**
- 找到对应的 TypeScript 源码
- 理解其实现逻辑、数据结构、边界情况

**Step 2: 伪代码分析（沟通前）**
- 用伪代码描述实现思路
- 标注关键点、潜在问题
- 等待用户确认或提问

**Step 3: 沟通确认**
- 向用户解释伪代码
- 讨论细节、边界情况
- 根据反馈调整方案

**Step 4: 实现代码**
- 在 `crates/ai/` 下实现
- 在 `examples/` 下提供可运行示例
- 确保 `cargo check` 和 `cargo build` 无 warning、无 error

**Step 5: 确认**
- 再次阅读 pi-mono 源码
- 对比实现是否正确

## 项目路径

- pi-mono: `<rs-mono>/refer/pi-mono/packages/`
- opencode: `<rs-mono>/refer/opencode/`
- rs-mono: `<rs-mono>/`

## 模块对照

| pi-mono | opencode | rs-mono |
|---------|----------|---------|
| packages/ai | - | crates/ai |
| packages/agent | src/agent/ | - (待设计) |
| packages/coding-agent | - | - (待设计) |

## Rust 实现要求

- 示例代码必须可直接运行: `cargo run --example <name>`
- 禁止任何 compiler warning
- 禁止任何 error
- 使用 kimi model 测试通过

## 对比原则

- **学习 pi-mono**：从 pi-mono 源码学习优秀设计思想
- **参考 opencode**：架构设计参考 opencode，不照搬
- **代码引用**：关键差异引用具体代码行号或函数名
- **影响分析**：说明每个缺失/差异对整体功能的影响
- **优先级建议**：根据功能重要性给出 P0/P1/P2 分级

优先级定义：
- **P0**: 架构基础，阻断核心功能
- **P1**: 重要功能，影响正确性
- **P2**: 优化功能，提升体验

## 注意事项

- 保持报告简洁，聚焦关键差异
- 使用表格提高可读性
- 用 🔴🟡🟢 等 emoji 标注严重程度
- 最后主动提出澄清问题