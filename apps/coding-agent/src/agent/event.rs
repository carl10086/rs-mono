use super::types::AgentEvent;

/// 事件处理器 trait
/// 用于订阅 Agent 事件，支持多个处理器并行通知
pub trait EventHandler: Send + Sync {
    /// 处理收到的事件
    fn on_event(&self, event: &AgentEvent);
}

/// 订阅 ID，用于取消订阅
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(u64);

/// 带 ID 的处理器封装
struct HandlerWithId {
    id: SubscriptionId,
    handler: Box<dyn EventHandler>,
}

/// 事件广播器
/// 支持多个 Handler 订阅，广播事件时通知所有订阅者
pub struct EventBroadcaster {
    /// 订阅的处理器列表
    handlers: Vec<HandlerWithId>,
    /// 下一个订阅 ID
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

    /// 订阅一个处理器，返回订阅 ID
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
    pub fn unsubscribe(&mut self, id: SubscriptionId) {
        self.handlers.retain(|h| h.id != id);
    }

    /// 广播事件到所有订阅的处理器
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
