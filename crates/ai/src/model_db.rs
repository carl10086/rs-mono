use crate::models::{Api, KnownProvider, Model, ModelCost, ModelInput};
use std::collections::HashMap;

pub struct ModelDb {
    models: HashMap<String, HashMap<String, Model>>,
}

impl ModelDb {
    pub fn new() -> Self {
        let mut models = HashMap::new();

        let kimi_models = Self::init_kimi_models();
        models.insert("kimi-coding".to_string(), kimi_models);

        Self { models }
    }

    fn init_kimi_models() -> HashMap<String, Model> {
        let mut models = HashMap::new();

        models.insert(
            "k2p5".to_string(),
            Model {
                id: "k2p5".to_string(),
                name: "Kimi K2.5".to_string(),
                api: Api::KimiCoding,
                provider: KnownProvider::KimiCoding,
                base_url: "https://api.kimi.com/coding".to_string(),
                reasoning: true,
                input: vec![ModelInput::Text, ModelInput::Image],
                cost: ModelCost::new(0.0, 0.0),
                context_window: 262144,
                max_tokens: 32768,
                headers: None,
            },
        );

        models.insert(
            "kimi-k2-turbo-preview".to_string(),
            Model {
                id: "kimi-k2-turbo-preview".to_string(),
                name: "Kimi K2 Turbo Preview".to_string(),
                api: Api::KimiCoding,
                provider: KnownProvider::KimiCoding,
                base_url: "https://api.kimi.com/coding".to_string(),
                reasoning: true,
                input: vec![ModelInput::Text],
                cost: ModelCost::new(0.0, 0.0),
                context_window: 262144,
                max_tokens: 32768,
                headers: None,
            },
        );

        models.insert(
            "kimi-k2-thinking".to_string(),
            Model {
                id: "kimi-k2-thinking".to_string(),
                name: "Kimi K2 Thinking".to_string(),
                api: Api::KimiCoding,
                provider: KnownProvider::KimiCoding,
                base_url: "https://api.kimi.com/coding".to_string(),
                reasoning: true,
                input: vec![ModelInput::Text],
                cost: ModelCost::new(0.0, 0.0),
                context_window: 262144,
                max_tokens: 32768,
                headers: None,
            },
        );

        models
    }

    pub fn get(&self, provider: &str, model_id: &str) -> Option<&Model> {
        self.models
            .get(provider)
            .and_then(|models| models.get(model_id))
    }

    pub fn get_by_provider(&self, provider: &str) -> Vec<&Model> {
        self.models
            .get(provider)
            .map(|models| models.values().collect())
            .unwrap_or_default()
    }
}

impl Default for ModelDb {
    fn default() -> Self {
        Self::new()
    }
}

pub fn get_model(provider: &str, model_id: &str) -> Option<Model> {
    static MODEL_DB: std::sync::OnceLock<ModelDb> = std::sync::OnceLock::new();
    MODEL_DB
        .get_or_init(ModelDb::new)
        .get(provider, model_id)
        .cloned()
}

pub fn get_kimi_model(model_id: &str) -> Option<Model> {
    get_model("kimi-coding", model_id)
}

pub fn calculate_cost(model: &Model, usage: &crate::types::Usage) -> crate::types::UsageCost {
    let mut cost = crate::types::UsageCost::new();

    cost.input = (model.cost.input / 1_000_000.0) * usage.input_tokens as f64;
    cost.output = (model.cost.output / 1_000_000.0) * usage.output_tokens as f64;
    cost.cache_read = (model.cost.cache_read / 1_000_000.0) * usage.cache_read_tokens as f64;
    cost.cache_write = (model.cost.cache_write / 1_000_000.0) * usage.cache_write_tokens as f64;
    cost.calculate();

    cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_kimi_model() {
        let model = get_kimi_model("kimi-k2-turbo-preview");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.id, "kimi-k2-turbo-preview");
        assert_eq!(model.api, Api::KimiCoding);
        assert!(model.reasoning);
    }

    #[test]
    fn test_get_kimi_model_k2p5() {
        let model = get_kimi_model("k2p5");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.id, "k2p5");
        assert!(model.reasoning);
    }

    #[test]
    fn test_get_model_not_found() {
        let model = get_model("kimi-coding", "non-existent-model");
        assert!(model.is_none());
    }

    #[test]
    fn test_calculate_cost() {
        let model = Model::new(
            "test",
            "Test",
            Api::KimiCoding,
            KnownProvider::KimiCoding,
            "https://test.com",
        )
        .with_cost(ModelCost::new(1.0, 2.0));

        let usage = crate::types::Usage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost: None,
        };

        let cost = calculate_cost(&model, &usage);
        assert!((cost.input - 1.0).abs() < 0.001);
        assert!((cost.output - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_model_db_singleton() {
        let db1 = ModelDb::new();
        let db2 = ModelDb::new();
        assert_eq!(
            db1.get_by_provider("kimi-coding").len(),
            db2.get_by_provider("kimi-coding").len()
        );
    }
}
