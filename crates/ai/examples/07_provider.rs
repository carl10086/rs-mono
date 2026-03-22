use ai::{
    providers::KimiProvider,
    models::{Api, Model, KnownProvider},
    types::{Context, Message, ContentBlock},
    client::{complete, register_provider},
    api_registry::get_env_api_key,
    Provider,
};

const ANSI_GREEN: &str = "\x1b[32m";

fn demo_provider_creation() {
    println!("{}━━━ Demo: Provider Creation ━━━", ANSI_GREEN);

    let api_key = std::env::var("KIMI_API_KEY")
        .expect("KIMI_API_KEY environment variable not set");

    let provider = KimiProvider::new("kimi-k2-turbo-preview", api_key)
        .with_thinking(1024);

    println!("Created KimiProvider:");
    println!("  name: {}", provider.name());
    println!("  api: {}", provider.api());
    println!("  model_id: {}", provider.model_id());
    println!();
}

fn demo_provider_registration() {
    println!("{}━━━ Demo: Provider Registration ━━━", ANSI_GREEN);

    let api_key = std::env::var("KIMI_API_KEY")
        .expect("KIMI_API_KEY environment variable not set");

    let provider = KimiProvider::new("kimi-k2-turbo-preview", api_key);
    register_provider(provider);

    println!("Provider registered successfully!");
    println!();
}

fn demo_env_api_key() {
    println!("{}━━━ Demo: Environment API Key Resolution ━━━", ANSI_GREEN);

    let providers = vec![
        "kimi-coding",
        "openai",
        "anthropic",
        "google",
        "groq",
    ];

    for provider in providers {
        let key = get_env_api_key(provider);
        match key {
            Some(_) => println!("  {}: {} (set)", provider, "✓"),
            None => println!("  {}: not set", provider),
        }
    }
    println!();
}

fn demo_model_api() {
    println!("{}━━━ Demo: Model API ━━━", ANSI_GREEN);

    let model = Model::new(
        "kimi-k2-turbo-preview",
        "Kimi K2 Turbo Preview",
        Api::KimiCoding,
        KnownProvider::KimiCoding,
        "https://api.kimi.com/coding/v1",
    );

    println!("Created model:");
    println!("  id: {}", model.id);
    println!("  name: {}", model.name);
    println!("  api: {}", model.api);
    println!("  provider: {}", model.provider);
    println!("  base_url: {}", model.base_url);
    println!();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n");
    demo_provider_creation();
    demo_env_api_key();
    demo_model_api();
    demo_provider_registration();

    println!("{}━━━ Demo: Complete via Client ━━━", ANSI_GREEN);

    let ctx = Context::new()
        .with_system_prompt("You are a helpful assistant.")
        .with_message(Message::user(vec![
            ContentBlock::text("Say 'Hello from client!' in exactly those words."),
        ]))
        .with_max_tokens(100);

    let model = Model::new(
        "kimi-k2-turbo-preview",
        "Kimi K2 Turbo Preview",
        Api::KimiCoding,
        KnownProvider::KimiCoding,
        "https://api.kimi.com/coding/v1",
    );

    println!("Sending complete() via client...");
    match complete(&model, &ctx).await {
        Ok(message) => {
            println!("\nResponse:");
            for block in &message.content {
                if let ContentBlock::Text(t) = block {
                    println!("  {}", t.text);
                }
            }
            println!("\n  Stop reason: {:?}", message.stop_reason);
            println!("  Usage: {:?}", message.usage);
        }
        Err(e) => {
            println!("\nError: {}", e);
        }
    }

    println!("\nDone!");
    Ok(())
}
