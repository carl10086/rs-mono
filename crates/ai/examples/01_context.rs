use ai::types::{ContentBlock, Context, Message, ThinkingBudgets, Tool};

const ANSI_GREEN: &str = "\x1b[32m";

fn demo_context_basic() {
    println!("{}━━━ Demo: Basic Context ━━━", ANSI_GREEN);

    let ctx = Context::new()
        .with_system_prompt("You are a helpful assistant.")
        .with_message(Message::user(vec![ContentBlock::text("Hello!")]));

    println!("Created context:");
    println!("  system_prompt: {:?}", ctx.system_prompt);
    println!("  messages: {}", ctx.messages.len());
    println!();
}

fn demo_context_with_options() {
    println!("{}━━━ Demo: Context with Options ━━━", ANSI_GREEN);

    let ctx = Context::new()
        .with_system_prompt("You are a coding assistant.")
        .with_message(Message::user(vec![ContentBlock::text(
            "Help me write a function.",
        )]))
        .with_max_tokens(2048)
        .with_temperature(0.7);

    println!("Created context with options:");
    println!("  system_prompt: {:?}", ctx.system_prompt);
    println!("  max_tokens: {:?}", ctx.max_tokens);
    println!("  temperature: {:?}", ctx.temperature);
    println!();
}

fn demo_context_with_tools() {
    println!("{}━━━ Demo: Context with Tools ━━━", ANSI_GREEN);

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

    let ctx = Context::new()
        .with_message(Message::user(vec![ContentBlock::text(
            "Run ls -la in /tmp",
        )]))
        .with_tool(bash_tool);

    println!("Created context with tools:");
    println!("  messages: {}", ctx.messages.len());
    println!("  tools: {}", ctx.tools.len());
    if let Some(tool) = ctx.tools.first() {
        println!("  tool name: {}", tool.name);
    }
    println!();
}

fn demo_context_builder_pattern() {
    println!("{}━━━ Demo: Builder Pattern ━━━", ANSI_GREEN);

    let ctx = Context::new()
        .with_system_prompt("You are helpful.")
        .with_message(Message::user(vec![ContentBlock::text("Hello")]))
        .with_message(Message::assistant(vec![ContentBlock::text("Hi there!")]))
        .with_message(Message::user(vec![ContentBlock::text("How are you?")]))
        .with_max_tokens(1024)
        .with_temperature(0.5);

    println!("Context with multiple messages:");
    println!("  messages count: {}", ctx.messages.len());
    for (i, msg) in ctx.messages.iter().enumerate() {
        println!("    {}: {:?}", i, msg.role);
    }
    println!();
}

fn demo_thinking_budgets() {
    println!("{}━━━ Demo: ThinkingBudgets ━━━", ANSI_GREEN);

    let budgets = ThinkingBudgets {
        minimal: Some(256),
        low: Some(512),
        medium: Some(1024),
        high: Some(2048),
    };

    let ctx = Context::new()
        .with_message(Message::user(vec![ContentBlock::text(
            "Solve this problem",
        )]))
        .with_thinking_budgets(budgets);

    println!("Context with thinking budgets: {:?}", ctx.thinking_budgets);
    println!();
}

fn main() {
    println!("\n");
    demo_context_basic();
    demo_context_with_options();
    demo_context_with_tools();
    demo_context_builder_pattern();
    demo_thinking_budgets();
    println!("Done!");
}
