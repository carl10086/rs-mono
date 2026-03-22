//! 事件广播系统
//!
//! 实现观察者模式，支持多个处理器订阅 Agent 事件。

use super::types::AgentEvent;

// ============================================================================
// 事件处理器接口
// ============================================================================

/// 事件处理器 trait
///
/// 用于订阅 Agent 事件的处理器接口。
/// 实现此 trait 来接收 Agent 运行过程中的各种事件。
pub trait EventHandler: Send + Sync {
    /// 处理收到的事件
    fn on_event(&self, event: &AgentEvent);
}

// ============================================================================
// 订阅管理
// ============================================================================

/// 订阅 ID
///
/// 用于取消订阅的唯一标识符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(u64);

/// 带 ID 的处理器封装
struct HandlerWithId {
    id: SubscriptionId,
    handler: Box<dyn EventHandler>,
}

// ============================================================================
// 事件广播器
// ============================================================================

/// 事件广播器
///
/// 维护一组事件处理器，将收到的事件广播给所有订阅者。
///
/// # 示例
///
/// ```ignore
/// let mut broadcaster = EventBroadcaster::new();
/// let id = broadcaster.subscribe(my_handler);
/// broadcaster.broadcast(&AgentEvent::AgentStart);
/// broadcaster.unsubscribe(id);
/// ```
pub struct EventBroadcaster {
    /// 已订阅的处理器列表
    handlers: Vec<HandlerWithId>,
    /// 下一个可用的订阅 ID
    next_id: u64,
}

impl EventBroadcaster {
    /// 创建新的广播器
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            next_id: 0,
        }
    }

    /// 获取当前订阅的处理器数量
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// 订阅一个处理器
    ///
    /// 返回订阅 ID，可用于取消订阅。
    pub fn subscribe<H: EventHandler + 'static>(&mut self, handler: H) -> SubscriptionId {
        let id = SubscriptionId(self.next_id);
        self.next_id += 1;

        self.handlers.push(HandlerWithId {
            id,
            handler: Box::new(handler),
        });

        id
    }

    /// 取消订阅
    ///
    /// 根据订阅 ID 移除对应的处理器。
    pub fn unsubscribe(&mut self, id: SubscriptionId) {
        self.handlers.retain(|h| h.id != id);
    }

    /// 广播事件
    ///
    /// 将事件发送给所有已订阅的处理器。
    pub fn broadcast(&self, event: &AgentEvent) {
        for handler in &self.handlers {
            handler.handler.on_event(event);
        }
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}
