use ai::client::register_provider;
use ai::model_db;
use ai::providers::KimiProvider;
use ai::types::{ContentBlock, Message};
use anyhow::Result;
use coding_agent::agent::{AgentEvent, AgentLoop, AgentLoopConfig, EventHandler};
use coding_agent::tools::ReadTool;
use std::sync::{Arc, Mutex};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const MAGENTA: &str = "\x1b[35m";

struct CliHandler {
    current_tool: Arc<Mutex<Option<String>>>,
}

impl CliHandler {
    fn new() -> Self {
        Self {
            current_tool: Arc::new(Mutex::new(None)),
        }
    }
}

impl EventHandler for CliHandler {
    fn on_event(&self, event: &AgentEvent) {
        match event {
            AgentEvent::AgentStart => {
                println!("{}>{} Starting agent...\n", CYAN, RESET);
            }
            AgentEvent::TurnStart => {}
            AgentEvent::MessageStart { .. } => {}
            AgentEvent::TextDelta { delta, .. } => {
                print!("{}", delta);
            }
            AgentEvent::TextEnd { .. } => {}
            AgentEvent::ThinkingStart { content_index: _ } => {
                print!("{}  ├─{} thinking", DIM, MAGENTA);
            }
            AgentEvent::ThinkingDelta { delta, .. } => {
                let truncated = delta.chars().take(15).collect::<String>();
                print!("{}", truncated);
            }
            AgentEvent::ThinkingEnd {
                content_index: _,
                content: _,
            } => {
                println!("{} done{}", GREEN, RESET);
            }
            AgentEvent::ToolCallStart { content_index: _ } => {
                print!("{}  ├─{} tool_call", DIM, YELLOW);
            }
            AgentEvent::ToolCallDelta { delta, .. } => {
                print!("{}", delta);
            }
            AgentEvent::ToolExecutionStart {
                tool_name,
                args,
                tool_call_id,
            } => {
                *self.current_tool.lock().unwrap() = Some(tool_name.clone());
                println!("{}  ├─{} Called tool {}{}", DIM, YELLOW, tool_name, RESET);
                println!("{}  │   └─{} input: {}{}", DIM, DIM, args, RESET);
                let _ = tool_call_id;
            }
            AgentEvent::ToolExecutionEnd {
                tool_name,
                is_error,
                ..
            } => {
                if *is_error {
                    println!("{}  │   └─{} error{}", DIM, RED, RESET);
                } else {
                    println!("{}  │   └─{} success{}", DIM, GREEN, RESET);
                }
                *self.current_tool.lock().unwrap() = None;
                let _ = tool_name;
            }
            AgentEvent::TurnEnd { tool_results, .. } => {
                for tr in tool_results {
                    for block in &tr.content {
                        if let ContentBlock::Text(t) = block {
                            let first_line = t
                                .text
                                .lines()
                                .next()
                                .unwrap_or("")
                                .chars()
                                .take(80)
                                .collect::<String>();
                            println!("{}  │   └─{} result: {}{}", DIM, GREEN, first_line, RESET);
                        }
                    }
                }
            }
            AgentEvent::AgentEnd { .. } => {
                println!("\n{}Done.{}", BOLD, RESET);
            }
            AgentEvent::Error { error } => {
                eprintln!("{}Error: {}{}", RED, error, RESET);
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().without_time())
        .with(EnvFilter::new("warn"))
        .init();

    let api_key = std::env::var("KIMI_API_KEY").expect("KIMI_API_KEY environment variable not set");

    let provider = KimiProvider::new("kimi-k2-turbo-preview", api_key).with_thinking(1024);
    register_provider(provider);

    let model = model_db::get_kimi_model("kimi-k2-turbo-preview")
        .unwrap()
        .with_reasoning(true);

    let mut agent = AgentLoop::new(AgentLoopConfig::new(model)).with_tools(vec![ReadTool::new()]);

    let handler = CliHandler::new();
    agent.subscribe(handler);

    let prompts = vec![Message::user(vec![ContentBlock::text(
        "List the files in the current directory. path=/Users/carlyu/soft/projects/rs-mono",
    )])];

    match agent.run(prompts).await {
        Ok(messages) => {
            println!("\n{}Final messages count: {}{}", DIM, messages.len(), RESET);
        }
        Err(e) => {
            eprintln!("{}Error: {}{}", RED, e, RESET);
        }
    }

    Ok(())
}
