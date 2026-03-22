use ai::types::{ContentBlock, Message};
use coding_agent::agent::{AgentEvent, AgentLoop, AgentLoopConfig, EventHandler};
use std::sync::{Arc, Mutex};

/// 测试用的事件处理器
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

    fn count(&self) -> usize {
        self.received.lock().unwrap().len()
    }

    fn clear(&self) {
        self.received.lock().unwrap().clear();
    }

    fn get_events(&self) -> Vec<AgentEvent> {
        self.received.lock().unwrap().clone()
    }
}

impl EventHandler for TestHandler {
    fn on_event(&self, event: &AgentEvent) {
        self.received.lock().unwrap().push(event.clone());
    }
}

/// 测试：创建 AgentLoop 时 broadcaster 为空
#[test]
fn test_agent_loop_has_empty_broadcaster() {
    let model = ai::model_db::get_kimi_model("kimi-k2-turbo-preview").unwrap();
    let agent = AgentLoop::new(AgentLoopConfig::new(model));

    assert_eq!(agent.broadcaster_handler_count(), 0);
}

/// 测试：订阅处理器
#[test]
fn test_agent_loop_subscribe_handler() {
    let model = ai::model_db::get_kimi_model("kimi-k2-turbo-preview").unwrap();
    let mut agent = AgentLoop::new(AgentLoopConfig::new(model));

    let handler = TestHandler::new();
    let _id = agent.subscribe(handler);

    assert_eq!(agent.broadcaster_handler_count(), 1);
}

/// 测试：取消订阅
#[test]
fn test_agent_loop_unsubscribe_handler() {
    let model = ai::model_db::get_kimi_model("kimi-k2-turbo-preview").unwrap();
    let mut agent = AgentLoop::new(AgentLoopConfig::new(model));

    let handler = TestHandler::new();
    let id = agent.subscribe(handler);

    assert_eq!(agent.broadcaster_handler_count(), 1);

    agent.unsubscribe(id);

    assert_eq!(agent.broadcaster_handler_count(), 0);
}

/// 测试：订阅后广播事件
#[test]
fn test_agent_loop_broadcast_to_handler() {
    let model = ai::model_db::get_kimi_model("kimi-k2-turbo-preview").unwrap();
    let mut agent = AgentLoop::new(AgentLoopConfig::new(model));

    let handler = TestHandler::new();
    agent.subscribe(handler.clone());

    // 通过 broadcaster 广播事件
    agent.broadcast(&AgentEvent::AgentStart);

    // 验证事件被接收
    let events = handler.get_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], AgentEvent::AgentStart));
}
