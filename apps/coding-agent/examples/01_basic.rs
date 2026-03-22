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
    thinking_buffer: Arc<Mutex<String>>,
}

impl CliHandler {
    fn new() -> Self {
        Self {
            thinking_buffer: Arc::new(Mutex::new(String::new())),
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
                self.thinking_buffer.lock().unwrap().clear();
            }
            AgentEvent::ThinkingDelta { delta, .. } => {
                self.thinking_buffer.lock().unwrap().push_str(&delta);
            }
            AgentEvent::ThinkingEnd { content_index: _, content: _ } => {
                let thinking = self.thinking_buffer.lock().unwrap().clone();
                let cleaned = thinking.replace('\n', " ").chars().take(60).collect::<String>();
                if !cleaned.is_empty() {
                    println!("{}  ├─{} 💭 {}{}", DIM, MAGENTA, cleaned, RESET);
                }
            }
            AgentEvent::ToolCallStart { content_index: _ } => {}
            AgentEvent::ToolCallDelta { delta: _, content_index: _ } => {}
            AgentEvent::ToolExecutionStart { tool_name, args, tool_call_id: _ } => {
                print!("\n");
                if let Some(obj) = args.as_object() {
                    let args_str: Vec<String> = obj.iter()
                        .filter(|(k, _)| *k != "_partial")
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect();
                    println!("{}  ├─{} 🔧 {} ({}){}", DIM, YELLOW, tool_name, args_str.join(", "), RESET);
                } else {
                    println!("{}  ├─{} 🔧 {}(){}", DIM, YELLOW, tool_name, RESET);
                }
            }
            AgentEvent::ToolExecutionEnd { tool_name: _, is_error, result, .. } => {
                if *is_error {
                    println!("{}  │   └─{} ✗ error{}", DIM, RED, RESET);
                } else {
                    let title = result.get("title").and_then(|t| t.as_str()).unwrap_or("done");
                    println!("{}  │   └─{} ✓ {} {}", DIM, GREEN, title, RESET);
                }
            }
            AgentEvent::TurnEnd { tool_results, .. } => {
                for tr in tool_results {
                    for block in &tr.content {
                        if let ContentBlock::Text(t) = block {
                            println!("\n{}  ├─{} 📤 Output:{}", DIM, GREEN, RESET);
                            let lines: Vec<&str> = t.text.lines().take(5).collect();
                            for line in lines {
                                println!("{}  │   │{} {}", DIM, RESET, line);
                            }
                            if t.text.lines().count() > 5 {
                                println!("{}  │   ... ({} more lines){}", DIM, t.text.lines().count() - 5, RESET);
                            }
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
        Ok(_messages) => {}
        Err(e) => {
            eprintln!("{}Error: {}{}", RED, e, RESET);
        }
    }

    Ok(())
}