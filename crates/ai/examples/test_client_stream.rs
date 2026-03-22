use ai::{
    providers::KimiProvider,
    types::Context,
    client::stream,
    register_provider,
    model_db,
};
use ai::types::{Message, ContentBlock};
use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("KIMI_API_KEY")
        .expect("KIMI_API_KEY not set");

    register_provider(KimiProvider::new("kimi-k2-turbo-preview", api_key));

    let model = model_db::get_kimi_model("kimi-k2-turbo-preview")
        .expect("Failed to get model");
    
    println!("Model: {:?}", model.id);
    println!("Model API: {:?}", model.api);

    let ctx = Context::new()
        .with_system_prompt("You are a helpful assistant.")
        .with_message(Message::user(vec![
            ContentBlock::text("Say 'Hello'"),
        ]));

    println!("Calling stream...");
    let mut stream_resp = stream(&model, &ctx).await?;
    println!("Got stream response");
    
    while let Some(result) = stream_resp.next().await {
        println!("Event: {:?}", result);
    }
    
    Ok(())
}
