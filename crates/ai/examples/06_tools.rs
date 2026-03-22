use ai::{
    providers::KimiProvider,
    types::{Context, Message, ContentBlock, Tool},
    Provider, StreamEvent,
};
use futures::StreamExt;
use std::io::Write;

const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_BLUE: &str = "\x1b[34m";
const ANSI_MAGENTA: &str = "\x1b[35m";

fn demo_tool_definition() {
    println!("{}━━━ Demo: Tool Definition ━━━", ANSI_GREEN);

    let bash_tool = Tool {
        name: "bash".to_string(),
        description: "Execute a bash command".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
                }
            },
            "required": ["command"]
        }),
    };

    println!("Created tool '{}':", bash_tool.name);
    println!("  description: {}", bash_tool.description);
    println!("  parameters: {}", serde_json::to_string_pretty(&bash_tool.parameters).unwrap());
    println!();
}

fn demo_tools_array() {
    println!("{}━━━ Demo: Multiple Tools ━━━", ANSI_GREEN);

    let tools = vec![
        Tool {
            name: "bash".to_string(),
            description: "Execute a bash command".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    }
                },
                "required": ["command"]
            }),
        },
        Tool {
            name: "read_file".to_string(),
            description: "Read contents of a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                },
                "required": ["path"]
            }),
        },
    ];

    println!("Created {} tools:", tools.len());
    for tool in &tools {
        println!("  - {}: {}", tool.name, tool.description);
    }
    println!();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    demo_tool_definition();
    demo_tools_array();

    let api_key = std::env::var("KIMI_API_KEY")
        .expect("KIMI_API_KEY environment variable not set");

    let provider = KimiProvider::new("kimi-k2-turbo-preview", api_key)
        .with_thinking(1024);

    let tools = vec![
        Tool {
            name: "bash".to_string(),
            description: "Execute a bash command".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    }
                },
                "required": ["command"]
            }),
        },
    ];

    let ctx = Context::new()
        .with_system_prompt("You are a helpful coding assistant.")
        .with_message(Message::user(vec![
            ContentBlock::text("What is the current working directory? Use the bash tool to run 'pwd'."),
        ]))
        .with_tools(tools);

    println!("{}━━━ Demo: Tool Call Streaming ━━━", ANSI_GREEN);
    println!("Sending request with tools...\n");

    let mut stream = provider.stream(&ctx).await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                match event {
                    StreamEvent::ThinkingStart { .. } => {
                        print!("\n{}┌─ Thinking{}\n", ANSI_MAGENTA, ANSI_GREEN);
                    }
                    StreamEvent::ThinkingDelta { delta, .. } => {
                        print!("{}", delta);
                        std::io::stdout().flush()?;
                    }
                    StreamEvent::ThinkingEnd { .. } => {
                        print!("{}\n", ANSI_GREEN);
                    }
                    StreamEvent::TextStart { .. } => {
                        print!("\n{}┌─ Response{}\n", ANSI_GREEN, ANSI_GREEN);
                    }
                    StreamEvent::TextDelta { delta, .. } => {
                        print!("{}", delta);
                        std::io::stdout().flush()?;
                    }
                    StreamEvent::TextEnd { .. } => {}
                    StreamEvent::ToolCallStart { content_index } => {
                        print!("\n{}┌─ {}Tool #{} Call{}\n", ANSI_MAGENTA, ANSI_BLUE, content_index, ANSI_GREEN);
                    }
                    StreamEvent::ToolCallDelta { delta, .. } => {
                        print!("{}", delta);
                        std::io::stdout().flush()?;
                    }
                    StreamEvent::ToolCallEnd { content_index, tool_call } => {
                        print!("{}\n", ANSI_GREEN);
                        println!("{}└─{}{}{}{}({})",
                            ANSI_MAGENTA,
                            ANSI_BLUE,
                            tool_call.name,
                            ANSI_GREEN,
                            content_index,
                            serde_json::to_string(&tool_call.arguments).unwrap_or_default()
                        );
                    }
                    StreamEvent::Done { reason, message } => {
                        println!("\n{}━━━ Done ━━━", ANSI_GREEN);
                        println!("  Stop reason: {:?}", reason);
                        println!("  Usage: {:?}", message.usage);
                    }
                    StreamEvent::Error { reason, error } => {
                        println!("\n{}━━━ Error ━━━", ANSI_GREEN);
                        println!("  Reason: {:?}", reason);
                        println!("  Error: {}", error);
                    }
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("\nStream error: {}", e);
            }
        }
    }

    println!("\nDone!");
    Ok(())
}
