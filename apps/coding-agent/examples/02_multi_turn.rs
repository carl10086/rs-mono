use anyhow::Result;
use coding_agent::agent::{AgentLoop, AgentLoopConfig, AgentEvent, EventHandler};
use coding_agent::tools::{BashTool, ReadTool, WriteTool, EditTool};
use ai::model_db;
use ai::providers::KimiProvider;
use ai::types::{ContentBlock, Message};
use ai::client::register_provider;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use std::sync::{Arc, Mutex};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const MAGENTA: &str = "\x1b[35m";
const BLUE: &str = "\x1b[34m";

struct CliHandler {
    current_tool: Arc<Mutex<Option<String>>>,
    turn_count: Arc<Mutex<u32>>,
}

impl CliHandler {
    fn new() -> Self {
        Self {
            current_tool: Arc::new(Mutex::new(None)),
            turn_count: Arc::new(Mutex::new(0)),
        }
    }
}

impl EventHandler for CliHandler {
    fn on_event(&self, event: &AgentEvent) {
        match event {
            AgentEvent::AgentStart => {
                println!("{}╔══════════════════════════════════════════════════════════════╗{}", CYAN, RESET);
                println!("{}║{}  🤖 Coding Agent - Multi-turn Conversation              {}║{}", CYAN, BOLD, CYAN, RESET);
                println!("{}╚══════════════════════════════════════════════════════════════╝{}", CYAN, RESET);
                println!();
            }
            AgentEvent::TurnStart => {
                let mut count = self.turn_count.lock().unwrap();
                *count += 1;
                println!("{}  ┌─ {}Turn {}{}", DIM, BLUE, count, RESET);
                println!("{}  │{}", DIM, RESET);
            }
            AgentEvent::MessageStart { .. } => {}
            AgentEvent::TextDelta { delta, .. } => {
                print!("{}  │   └─{} {}", DIM, GREEN, delta);
            }
            AgentEvent::TextEnd { .. } => {
                println!();
            }
            AgentEvent::ThinkingStart { content_index: _ } => {
                print!("{}  │   └─ {}Thinking{} ", DIM, MAGENTA, RESET);
            }
            AgentEvent::ThinkingDelta { delta, .. } => {
                let truncated = delta.chars().take(20).collect::<String>();
                print!("{}", truncated);
            }
            AgentEvent::ThinkingEnd { content_index: _, content: _ } => {
                println!(" {}✓{}", GREEN, RESET);
            }
            AgentEvent::ToolCallStart { content_index: _ } => {
                print!("{}  │   └─ {}Calling tool{} ", DIM, YELLOW, RESET);
            }
            AgentEvent::ToolCallDelta { delta, .. } => {
                print!("{}", delta);
            }
            AgentEvent::ToolCallEnd { content_index: _, tool_call } => {
                println!("{} ({}){}", DIM, tool_call.name, RESET);
            }
            AgentEvent::ToolExecutionStart { tool_name, args, tool_call_id } => {
                *self.current_tool.lock().unwrap() = Some(tool_name.clone());
                let args_str = if args.is_null() {
                    "{}".to_string()
                } else {
                    serde_json::to_string_pretty(&args).unwrap_or_else(|_| args.to_string())
                };
                println!("{}  │   ├─{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", DIM, YELLOW, RESET);
                println!("{}  │   │{} Tool: {}{}", DIM, YELLOW, tool_name, RESET);
                let _ = tool_call_id;
                let _ = args_str;
            }
            AgentEvent::ToolExecutionEnd { tool_call_id: _, tool_name, is_error, result } => {
                let status = if *is_error {
                    format!("{}✗ ERROR{}", RED, RESET)
                } else {
                    format!("{}✓ SUCCESS{}", GREEN, RESET)
                };
                println!("{}  │   │{} Status: {}{}", DIM, if *is_error { RED } else { GREEN }, status, RESET);
                if let Ok(result_obj) = serde_json::from_value::<serde_json::Value>(result.clone()) {
                    if let Some(title) = result_obj.get("title") {
                        println!("{}  │   │{} Title: {}{}", DIM, DIM, title, RESET);
                    }
                }
                println!("{}  │   └─{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", DIM, YELLOW, RESET);
                *self.current_tool.lock().unwrap() = None;
                let _ = tool_name;
            }
            AgentEvent::TurnEnd { tool_results, .. } => {
                for tr in tool_results {
                    for block in &tr.content {
                        if let ContentBlock::Text(t) = block {
                            let lines: Vec<&str> = t.text.lines().collect();
                            let preview = if lines.len() > 3 {
                                format!("{}\n{}", lines.iter().take(3).map(|s| *s).collect::<Vec<_>>().join("\n"), "...")
                            } else {
                                t.text.clone()
                            }.chars().take(150).collect::<String>();
                            println!("{}  │   └─{} Result: {}{}", DIM, GREEN, preview, RESET);
                        }
                    }
                }
                println!("{}  │{}", DIM, RESET);
                println!("{}  └─{}Turn End{}", DIM, BLUE, RESET);
                println!();
            }
            AgentEvent::AgentEnd { .. } => {
                println!("{}╔══════════════════════════════════════════════════════════════╗{}", CYAN, RESET);
                println!("{}║{}  ✅ Conversation Complete                                    {}║{}", GREEN, BOLD, CYAN, RESET);
                println!("{}╚══════════════════════════════════════════════════════════════╝{}", CYAN, RESET);
            }
            AgentEvent::Error { error } => {
                eprintln!("{}  ╳ Error: {}{}", RED, error, RESET);
            }
            _ => {}
        }
    }
}

fn build_messages(history: &[Message], new_prompt: String) -> Vec<Message> {
    let mut messages = history.to_vec();
    messages.push(Message::user(vec![ContentBlock::text(new_prompt)]));
    messages
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().without_time())
        .with(EnvFilter::new("warn"))
        .init();

    let api_key = std::env::var("KIMI_API_KEY")
        .expect("KIMI_API_KEY environment variable not set");
    
    let provider = KimiProvider::new("kimi-k2-turbo-preview", api_key);
    register_provider(provider);

    let model = model_db::get_kimi_model("kimi-k2-turbo-preview")
        .expect("Failed to get model");

    let system_prompt = r#"You are a coding assistant. You MUST use tools to accomplish tasks.

Available tools:
- bash: Execute bash commands (args: command, description, timeout?, workdir?)
- read: Read files (args: filePath, offset?, limit?)
- write: Write files (args: filePath, content)
- edit: Edit files (args: filePath, oldString, newString)

IMPORTANT RULES:
1. When asked to create a file, use the write tool
2. When asked to modify a file, use the edit tool (do NOT rewrite the whole file)
3. Always verify your changes by running the appropriate command
4. Do NOT just describe what you would do - actually do it using tools"#;

    let mut agent = AgentLoop::new(AgentLoopConfig::new(model).with_system_prompt(system_prompt))
        .with_tools(vec![BashTool::new()])
        .with_tools(vec![ReadTool::new()])
        .with_tools(vec![WriteTool::new()])
        .with_tools(vec![EditTool::new()]);

    let handler = CliHandler::new();
    agent.subscribe(handler);

    let workdir = "/Users/carlyu/soft/tmp/hello";
    
    let mut conversation_history: Vec<Message> = Vec::new();

    println!("{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", BOLD, RESET);
    println!("{}  📁 Working Directory: {}{}", BOLD, workdir, RESET);
    println!("{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", BOLD, RESET);
    println!();

    println!("{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", DIM, RESET);
    println!("{}  Step 1: Create a Python hello-world script{}", DIM, RESET);
    println!("{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", DIM, RESET);
    
    let prompt1 = format!(r#"Create a Python hello-world script at {}/hello.py that prints "Hello, World!" when run."#, workdir);
    let messages1 = build_messages(&conversation_history, prompt1);
    
    match agent.run(messages1).await {
        Ok(messages) => {
            conversation_history = messages;
        }
        Err(e) => {
            eprintln!("{}Error: {}{}", RED, e, RESET);
        }
    }

    println!("{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", DIM, RESET);
    println!("{}  Step 2: Modify the script to also print current time{}", DIM, RESET);
    println!("{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", DIM, RESET);

    let prompt2 = format!(r#"Now modify {}/hello.py to also print the current date and time when the script runs. Use the datetime module."#, workdir);
    let messages2 = build_messages(&conversation_history, prompt2);
    
    match agent.run(messages2).await {
        Ok(messages) => {
            conversation_history = messages;
        }
        Err(e) => {
            eprintln!("{}Error: {}{}", RED, e, RESET);
        }
    }

    println!("{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", DIM, RESET);
    println!("{}  Step 3: Modify the script to print the PID of the highest CPU process{}", DIM, RESET);
    println!("{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", DIM, RESET);

    let prompt3 = format!(r#"Now modify {}/hello.py again to also print the process ID (PID) of the process that is currently using the most CPU. On macOS, you can use `ps aux` to get this information. The output should show the PID with the highest CPU percentage."#, workdir);
    let messages3 = build_messages(&conversation_history, prompt3);
    
    match agent.run(messages3).await {
        Ok(messages) => {
            conversation_history = messages;
            println!("\n{}Final conversation had {} messages{}", DIM, conversation_history.len(), RESET);
        }
        Err(e) => {
            eprintln!("{}Error: {}{}", RED, e, RESET);
        }
    }

    Ok(())
}
