use crate::types::{AssistantMessage, StopReason, ToolCall};

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Start {
        partial: AssistantMessage,
    },
    TextStart {
        content_index: usize,
    },
    TextDelta {
        content_index: usize,
        delta: String,
    },
    TextEnd {
        content_index: usize,
        content: String,
    },
    ThinkingStart {
        content_index: usize,
    },
    ThinkingDelta {
        content_index: usize,
        delta: String,
    },
    ThinkingEnd {
        content_index: usize,
        content: String,
    },
    ToolCallStart {
        content_index: usize,
    },
    ToolCallDelta {
        content_index: usize,
        delta: String,
    },
    ToolCallEnd {
        content_index: usize,
        tool_call: ToolCall,
    },
    Done {
        reason: StopReason,
        message: AssistantMessage,
    },
    Error {
        reason: StopReason,
        error: String,
    },
}

impl StreamEvent {
    pub fn is_done(&self) -> bool {
        matches!(self, StreamEvent::Done { .. } | StreamEvent::Error { .. })
    }

    pub fn done_reason(&self) -> Option<StopReason> {
        match self {
            StreamEvent::Done { reason, .. } => Some(*reason),
            StreamEvent::Error { reason, .. } => Some(*reason),
            _ => None,
        }
    }
}
