pub mod agent;
pub mod tools;

use agent::{AgentLoop, AgentLoopConfig};
use ai::model_db;
use ai::providers::KimiProvider;
use ai::types::Message;
use ai::client::register_provider;
use tools::ReadTool;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting coding-agent...");

    let api_key = std::env::var("KIMI_API_KEY")
        .expect("KIMI_API_KEY environment variable not set");
    
    let provider = KimiProvider::new("kimi-k2-turbo-preview", api_key);
    register_provider(provider);

    let model = model_db::get_kimi_model("kimi-k2-turbo-preview")
        .expect("Failed to get model");

    let mut agent = AgentLoop::new(AgentLoopConfig::new(model))
        .with_tools(vec![ReadTool::new()]);

    let prompts = vec![Message::user(vec![ai::types::ContentBlock::text(
        "List the files in the current directory using the read tool, path=/Users/carlyu/soft/projects/rs-mono",
    )])];

    match agent.run(prompts).await {
        Ok(events) => {
            println!("\n=== Events ===");
            for event in &events {
                println!("{:?}", event);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}
