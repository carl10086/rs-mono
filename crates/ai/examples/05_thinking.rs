use ai::{
    providers::KimiProvider,
    types::{Context, Message, ContentBlock, ThinkingLevel},
    Provider, StreamEvent,
};
use futures::StreamExt;
use std::io::Write;

const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_YELLOW: &str = "\x1b[33m";

fn demo_thinking_levels() {
    println!("{}━━━ Demo: Thinking Levels ━━━", ANSI_GREEN);

    println!("Available ThinkingLevel values:");
    println!("  ThinkingLevel::Minimal");
    println!("  ThinkingLevel::Low");
    println!("  ThinkingLevel::Medium");
    println!("  ThinkingLevel::High");
    println!("  ThinkingLevel::Xhigh");
    println!();
}

fn demo_context_with_thinking() {
    println!("{}━━━ Demo: Context with Thinking ━━━", ANSI_GREEN);

    let ctx = Context::new()
        .with_system_prompt("You are a helpful assistant.")
        .with_message(Message::user(vec![
            ContentBlock::text("What is 2 + 2?"),
        ]))
        .with_thinking(ThinkingLevel::High);

    println!("Context with thinking enabled:");
    println!("  thinking: {:?}", ctx.thinking);
    println!();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    demo_thinking_levels();
    demo_context_with_thinking();

    let api_key = std::env::var("KIMI_API_KEY")
        .expect("KIMI_API_KEY environment variable not set");

    let provider = KimiProvider::new("kimi-k2-turbo-preview", api_key)
        .with_thinking(1024);

    let ctx = Context::new()
        .with_system_prompt("You are a helpful assistant.")
        .with_message(Message::user(vec![
            ContentBlock::text("Explain why the sky is blue. Be concise."),
        ]))
        .with_thinking(ThinkingLevel::Medium);

    println!("{}━━━ Demo: Streaming with Thinking ━━━", ANSI_GREEN);
    println!("Sending request with thinking enabled...\n");

    let mut stream = provider.stream(&ctx).await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                match event {
                    StreamEvent::ThinkingStart { .. } => {
                        print!("\n{}┌─ Thinking{}", ANSI_YELLOW, ANSI_GREEN);
                        std::io::stdout().flush()?;
                    }
                    StreamEvent::ThinkingDelta { delta, .. } => {
                        print!("{}", delta);
                        std::io::stdout().flush()?;
                    }
                    StreamEvent::ThinkingEnd { .. } => {
                        print!("{}\n", ANSI_GREEN);
                        print!("├─ Response: ");
                        std::io::stdout().flush()?;
                    }
                    StreamEvent::TextStart { .. } => {}
                    StreamEvent::TextDelta { delta, .. } => {
                        print!("{}", delta);
                        std::io::stdout().flush()?;
                    }
                    StreamEvent::TextEnd { .. } => {}
                    StreamEvent::Done { reason, message } => {
                        println!("\n\n{}━━━ Done ━━━", ANSI_GREEN);
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
