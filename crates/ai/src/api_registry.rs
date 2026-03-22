use crate::models::Api;
use crate::provider::Provider;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct ApiProviderRegistry {
    providers: HashMap<String, Box<dyn Provider>>,
}

impl ApiProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn register<P: Provider + 'static>(&mut self, provider: P) {
        let api = provider.api();
        self.providers
            .insert(api.as_str().to_string(), Box::new(provider));
    }

    pub fn get(&self, api: &Api) -> Option<&dyn Provider> {
        self.providers.get(api.as_str()).map(|p| p.as_ref())
    }

    pub fn get_all_apis(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.providers.clear();
    }
}

impl Default for ApiProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn get_env_api_key(provider: &str) -> Option<String> {
    let env_map: HashMap<&str, &str> = HashMap::from([
        ("openai", "OPENAI_API_KEY"),
        ("azure-openai-responses", "AZURE_OPENAI_API_KEY"),
        ("google", "GEMINI_API_KEY"),
        ("anthropic", "ANTHROPIC_API_KEY"),
        ("groq", "GROQ_API_KEY"),
        ("cerebras", "CEREBRAS_API_KEY"),
        ("xai", "XAI_API_KEY"),
        ("openrouter", "OPENROUTER_API_KEY"),
        ("vercel-ai-gateway", "AI_GATEWAY_API_KEY"),
        ("zai", "ZAI_API_KEY"),
        ("mistral", "MISTRAL_API_KEY"),
        ("minimax", "MINIMAX_API_KEY"),
        ("minimax-cn", "MINIMAX_CN_API_KEY"),
        ("huggingface", "HF_TOKEN"),
        ("opencode", "OPENCODE_API_KEY"),
        ("opencode-go", "OPENCODE_API_KEY"),
        ("kimi-coding", "KIMI_API_KEY"),
    ]);

    env_map
        .get(provider)
        .and_then(|key| std::env::var(key).ok())
}

static REGISTRY: std::sync::OnceLock<Mutex<ApiProviderRegistry>> = std::sync::OnceLock::new();

pub fn get_registry() -> &'static Mutex<ApiProviderRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(ApiProviderRegistry::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let registry = ApiProviderRegistry::new();
        assert!(registry.get(&Api::KimiCoding).is_none());
    }
}
