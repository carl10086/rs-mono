use ai::{
    providers::KimiProvider,
    types::{Context, Message, ContentBlock},
    Provider, StreamEvent,
};
use futures::StreamExt;
use std::io::Write;

const ANSI_GREEN: &str = "\x1b[32m";

fn demo_stream_basic() {
    println!("{}━━━ Demo: Basic Stream (sync) ━━━", ANSI_GREEN);
    println!("This demonstrates a simple streaming call without tools.\n");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    demo_stream_basic();
    
    let api_key = std::env::var("KIMI_API_KEY")
        .expect("KIMI_API_KEY environment variable not set");

    let provider = KimiProvider::new("kimi-k2-turbo-preview", api_key);

    let ctx = Context::new()
        .with_system_prompt("You are a helpful assistant.")
        .with_message(Message::user(vec![
            ContentBlock::text("Say 'Hello World' in exactly those words."),
        ]));

    println!("Sending streaming request...\n");

    let mut stream = provider.stream(&ctx).await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                match event {
                    StreamEvent::Start { .. } => {}
                    StreamEvent::TextStart { .. } => {
                        print!("Response: ");
                    }
                    StreamEvent::TextDelta { delta, .. } => {
                        print!("{}", delta);
                        std::io::stdout().flush()?;
                    }
                    StreamEvent::TextEnd { .. } => {}
                    StreamEvent::ThinkingStart { .. } => {}
                    StreamEvent::ThinkingDelta { delta, .. } => {
                        print!("[Thinking: {}...]", &delta[..delta.len().min(20)]);
                        std::io::stdout().flush()?;
                    }
                    StreamEvent::ThinkingEnd { .. } => {}
                    StreamEvent::ToolCallStart { .. } => {}
                    StreamEvent::ToolCallDelta { delta: _, .. } => {}
                    StreamEvent::ToolCallEnd { .. } => {}
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
