use ai::{
    model_db::{calculate_cost, get_kimi_model},
    models::{Api, KnownProvider, Model, ModelCost},
    types::Usage,
};

const ANSI_GREEN: &str = "\x1b[32m";

fn demo_model_creation() {
    println!("{}━━━ Demo: Model Creation ━━━", ANSI_GREEN);

    let model = Model::new(
        "kimi-k2-turbo-preview",
        "Kimi K2 Turbo Preview",
        Api::KimiCoding,
        KnownProvider::KimiCoding,
        "https://api.kimi.com/coding/v1",
    )
    .with_reasoning(true)
    .with_context_window(128000)
    .with_max_tokens(8192)
    .with_cost(ModelCost::new(0.0, 0.0));

    println!("Model created:");
    println!("  id: {}", model.id);
    println!("  name: {}", model.name);
    println!("  api: {}", model.api);
    println!("  provider: {}", model.provider);
    println!("  base_url: {}", model.base_url);
    println!("  reasoning: {}", model.reasoning);
    println!("  context_window: {}", model.context_window);
    println!("  max_tokens: {}", model.max_tokens);
    println!();
}

fn demo_model_db() {
    println!("{}━━━ Demo: Model Database ━━━", ANSI_GREEN);

    if let Some(model) = get_kimi_model("kimi-k2-turbo-preview") {
        println!("Found model '{}':", model.id);
        println!("  name: {}", model.name);
        println!("  api: {}", model.api);
        println!("  context_window: {}", model.context_window);
        println!("  max_tokens: {}", model.max_tokens);
        println!("  reasoning: {}", model.reasoning);
    }

    if let Some(model) = get_kimi_model("k2p5") {
        println!("\nFound model '{}':", model.id);
        println!("  name: {}", model.name);
        println!("  reasoning: {}", model.reasoning);
    }
    println!();
}

fn demo_calculate_cost() {
    println!("{}━━━ Demo: Cost Calculation ━━━", ANSI_GREEN);

    if let Some(model) = get_kimi_model("kimi-k2-turbo-preview") {
        let usage = Usage {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: 200,
            cache_write_tokens: 50,
            cost: None,
        };

        let cost = calculate_cost(&model, &usage);
        println!("Usage: 1000 input, 500 output, 200 cache_read, 50 cache_write");
        println!("Cost breakdown:");
        println!("  input: ${:.6}", cost.input);
        println!("  output: ${:.6}", cost.output);
        println!("  cache_read: ${:.6}", cost.cache_read);
        println!("  cache_write: ${:.6}", cost.cache_write);
        println!("  total: ${:.6}", cost.total);
    }
    println!();
}

fn demo_model_cost_creation() {
    println!("{}━━━ Demo: ModelCost Builder ━━━", ANSI_GREEN);

    let cost = ModelCost::new(0.5, 1.5)
        .with_cache_read(0.1)
        .with_cache_write(0.2);

    println!("ModelCost created:");
    println!("  input: {}", cost.input);
    println!("  output: {}", cost.output);
    println!("  cache_read: {}", cost.cache_read);
    println!("  cache_write: {}", cost.cache_write);
    println!();
}

fn main() {
    println!("\n");
    demo_model_creation();
    demo_model_db();
    demo_calculate_cost();
    demo_model_cost_creation();
    println!("Done!");
}
