use ai::types::{ContentBlock, Message};

const ANSI_GREEN: &str = "\x1b[32m";

fn demo_user_message() {
    println!("{}━━━ Demo: User Message ━━━", ANSI_GREEN);

    let msg = Message::user(vec![ContentBlock::text("Hello, how are you?")]);

    println!("Role: {:?}", msg.role);
    println!("Text: {}", msg.text_content().unwrap());
    println!();
}

fn demo_assistant_message() {
    println!("{}━━━ Demo: Assistant Message ━━━", ANSI_GREEN);

    let msg = Message::assistant(vec![ContentBlock::text("I'm doing well, thank you!")]);

    println!("Role: {:?}", msg.role);
    println!("Text: {}", msg.text_content().unwrap());
    println!();
}

fn demo_tool_result_message() {
    println!("{}━━━ Demo: Tool Result Message ━━━", ANSI_GREEN);

    let msg = Message::tool_result(
        "tool_abc123",
        "bash",
        vec![ContentBlock::text("/home/user/projects")],
    );

    println!("Role: {:?}", msg.role);
    println!("Tool call ID: {:?}", msg.tool_call_id);
    println!("Tool name: {:?}", msg.name);
    println!("Result text: {}", msg.text_content().unwrap());
    println!();
}

fn demo_system_message() {
    println!("{}━━━ Demo: System Message ━━━", ANSI_GREEN);

    let msg = Message::system("You are a helpful coding assistant.");

    println!("Role: {:?}", msg.role);
    println!("Text: {}", msg.text_content().unwrap());
    println!();
}

fn demo_multi_content_message() {
    println!("{}━━━ Demo: Multi-Content Message ━━━", ANSI_GREEN);

    let msg = Message::user(vec![
        ContentBlock::text("Here's what I found:"),
        ContentBlock::thinking("Let me analyze this..."),
    ]);

    println!("Role: {:?}", msg.role);
    println!("Content blocks: {}", msg.content.len());
    for (i, block) in msg.content.iter().enumerate() {
        match block {
            ContentBlock::Text(t) => println!("  {}: Text - {}", i, t.text),
            ContentBlock::Thinking(t) => println!("  {}: Thinking - {}", i, t.thinking),
            _ => println!("  {}: Other", i),
        }
    }
    println!();
}

fn demo_assistant_with_tool_call() {
    println!("{}━━━ Demo: Assistant with Tool Call ━━━", ANSI_GREEN);

    let msg = Message::assistant(vec![
        ContentBlock::text("Let me run that command for you."),
        ContentBlock::tool_call(
            "tool_xyz789",
            "bash",
            serde_json::json!({"command": "ls -la"}),
        ),
    ]);

    println!("Role: {:?}", msg.role);
    println!("Content blocks: {}", msg.content.len());
    for (i, block) in msg.content.iter().enumerate() {
        match block {
            ContentBlock::Text(t) => println!("  {}: Text - {}", i, t.text),
            ContentBlock::ToolCall(tc) => println!("  {}: ToolCall - {}()", i, tc.name),
            _ => println!("  {}: Other", i),
        }
    }
    println!();
}

fn demo_message_roles() {
    println!("{}━━━ Demo: Role Types ━━━", ANSI_GREEN);

    println!("Available roles:");
    println!("  Role::User - Messages from the user");
    println!("  Role::Assistant - Messages from the assistant");
    println!("  Role::ToolResult - Results from tool execution");
    println!("  Role::System - System-level instructions");
    println!();
}

fn demo_content_block_types() {
    println!("{}━━━ Demo: Content Block Types ━━━", ANSI_GREEN);

    println!("Available content block types:");
    println!("  ContentBlock::Text - Plain text content");
    println!("  ContentBlock::Thinking - Internal thinking process");
    println!("  ContentBlock::ToolCall - Request to call a tool");
    println!("  ContentBlock::Image - Image content (base64)");
    println!();
}

fn main() {
    println!("\n");
    demo_message_roles();
    demo_content_block_types();
    demo_user_message();
    demo_assistant_message();
    demo_tool_result_message();
    demo_system_message();
    demo_multi_content_message();
    demo_assistant_with_tool_call();
    println!("Done!");
}
