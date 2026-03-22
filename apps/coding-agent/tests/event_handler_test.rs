use ai::types::{ContentBlock, Message, Role};
use coding_agent::agent::{AgentEvent, EventBroadcaster, EventHandler};
use std::sync::{Arc, Mutex};

/// 测试用的简单事件处理器
#[derive(Clone)]
struct TestHandler {
    /// 收集收到的事件
    received: Arc<Mutex<Vec<AgentEvent>>>,
}

impl TestHandler {
    fn new() -> Self {
        Self {
            received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 获取收到的事件数量
    fn count(&self) -> usize {
        self.received.lock().unwrap().len()
    }

    /// 获取最后收到的事件
    fn last_event(&self) -> Option<AgentEvent> {
        self.received.lock().unwrap().last().cloned()
    }
}

impl EventHandler for TestHandler {
    fn on_event(&self, event: &AgentEvent) {
        self.received.lock().unwrap().push(event.clone());
    }
}

/// 测试：创建广播器
#[test]
fn test_broadcaster_creation() {
    let broadcaster = EventBroadcaster::new();
    assert_eq!(broadcaster.handler_count(), 0);
}

/// 测试：订阅一个处理器
#[test]
fn test_subscribe_single_handler() {
    let mut broadcaster = EventBroadcaster::new();
    let handler = TestHandler::new();

    broadcaster.subscribe(handler);

    assert_eq!(broadcaster.handler_count(), 1);
}

/// 测试：订阅多个处理器
#[test]
fn test_subscribe_multiple_handlers() {
    let mut broadcaster = EventBroadcaster::new();
    let handler1 = TestHandler::new();
    let handler2 = TestHandler::new();

    broadcaster.subscribe(handler1);
    broadcaster.subscribe(handler2);

    assert_eq!(broadcaster.handler_count(), 2);
}

/// 测试：广播事件到单个处理器
#[test]
fn test_broadcast_single_handler() {
    let mut broadcaster = EventBroadcaster::new();
    let handler = TestHandler::new();

    broadcaster.subscribe(handler.clone());

    let event = AgentEvent::AgentStart;
    broadcaster.broadcast(&event);

    assert_eq!(handler.count(), 1);
    assert!(matches!(handler.last_event(), Some(AgentEvent::AgentStart)));
}

/// 测试：广播事件到多个处理器
#[test]
fn test_broadcast_multiple_handlers() {
    let mut broadcaster = EventBroadcaster::new();
    let handler1 = TestHandler::new();
    let handler2 = TestHandler::new();

    broadcaster.subscribe(handler1.clone());
    broadcaster.subscribe(handler2.clone());

    let event = AgentEvent::AgentStart;
    broadcaster.broadcast(&event);

    assert_eq!(handler1.count(), 1);
    assert_eq!(handler2.count(), 1);
}

/// 测试：取消订阅
#[test]
fn test_unsubscribe() {
    let mut broadcaster = EventBroadcaster::new();
    let handler = TestHandler::new();
    let id = broadcaster.subscribe(handler);

    assert_eq!(broadcaster.handler_count(), 1);

    broadcaster.unsubscribe(id);

    assert_eq!(broadcaster.handler_count(), 0);
}

/// 测试：发送不同类型的事件
#[test]
fn test_broadcast_different_event_types() {
    let mut broadcaster = EventBroadcaster::new();
    let handler = TestHandler::new();

    broadcaster.subscribe(handler.clone());

    // 广播 AgentStart
    broadcaster.broadcast(&AgentEvent::AgentStart);

    // 广播 TurnEnd（需要构造完整的消息）
    let msg = Message {
        role: Role::Assistant,
        content: vec![ContentBlock::Text(ai::types::TextContent {
            text: "test".to_string(),
        })],
        name: None,
        tool_call_id: None,
    };
    broadcaster.broadcast(&AgentEvent::TurnEnd {
        message: msg,
        tool_results: vec![],
    });

    assert_eq!(handler.count(), 2);
}
