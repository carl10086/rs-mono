use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Api {
    #[serde(rename = "openai-completions")]
    OpenaiCompletions,
    #[serde(rename = "mistral-conversations")]
    MistralConversations,
    #[serde(rename = "openai-responses")]
    OpenaiResponses,
    #[serde(rename = "azure-openai-responses")]
    AzureOpenaiResponses,
    #[serde(rename = "openai-codex-responses")]
    OpenaiCodexResponses,
    #[serde(rename = "anthropic-messages")]
    AnthropicMessages,
    #[serde(rename = "bedrock-converse-stream")]
    BedrockConverseStream,
    #[serde(rename = "google-generative-ai")]
    GoogleGenerativeAi,
    #[serde(rename = "google-gemini-cli")]
    GoogleGeminiCli,
    #[serde(rename = "google-vertex")]
    GoogleVertex,
    #[serde(rename = "kimi-coding")]
    KimiCoding,
}

impl Api {
    pub fn as_str(&self) -> &'static str {
        match self {
            Api::OpenaiCompletions => "openai-completions",
            Api::MistralConversations => "mistral-conversations",
            Api::OpenaiResponses => "openai-responses",
            Api::AzureOpenaiResponses => "azure-openai-responses",
            Api::OpenaiCodexResponses => "openai-codex-responses",
            Api::AnthropicMessages => "anthropic-messages",
            Api::BedrockConverseStream => "bedrock-converse-stream",
            Api::GoogleGenerativeAi => "google-generative-ai",
            Api::GoogleGeminiCli => "google-gemini-cli",
            Api::GoogleVertex => "google-vertex",
            Api::KimiCoding => "kimi-coding",
        }
    }
}

impl std::fmt::Display for Api {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KnownProvider {
    #[serde(rename = "amazon-bedrock")]
    AmazonBedrock,
    Anthropic,
    Google,
    #[serde(rename = "google-gemini-cli")]
    GoogleGeminiCli,
    #[serde(rename = "google-antigravity")]
    GoogleAntigravity,
    #[serde(rename = "google-vertex")]
    GoogleVertex,
    Openai,
    #[serde(rename = "azure-openai-responses")]
    AzureOpenaiResponses,
    #[serde(rename = "openai-codex")]
    OpenaiCodex,
    #[serde(rename = "github-copilot")]
    GithubCopilot,
    Xai,
    Groq,
    Cerebras,
    Openrouter,
    #[serde(rename = "vercel-ai-gateway")]
    VercelAiGateway,
    Zai,
    Mistral,
    Minimax,
    #[serde(rename = "minimax-cn")]
    MinimaxCn,
    Huggingface,
    Opencode,
    #[serde(rename = "opencode-go")]
    OpencodeGo,
    #[serde(rename = "kimi-coding")]
    KimiCoding,
}

impl KnownProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            KnownProvider::AmazonBedrock => "amazon-bedrock",
            KnownProvider::Anthropic => "anthropic",
            KnownProvider::Google => "google",
            KnownProvider::GoogleGeminiCli => "google-gemini-cli",
            KnownProvider::GoogleAntigravity => "google-antigravity",
            KnownProvider::GoogleVertex => "google-vertex",
            KnownProvider::Openai => "openai",
            KnownProvider::AzureOpenaiResponses => "azure-openai-responses",
            KnownProvider::OpenaiCodex => "openai-codex",
            KnownProvider::GithubCopilot => "github-copilot",
            KnownProvider::Xai => "xai",
            KnownProvider::Groq => "groq",
            KnownProvider::Cerebras => "cerebras",
            KnownProvider::Openrouter => "openrouter",
            KnownProvider::VercelAiGateway => "vercel-ai-gateway",
            KnownProvider::Zai => "zai",
            KnownProvider::Mistral => "mistral",
            KnownProvider::Minimax => "minimax",
            KnownProvider::MinimaxCn => "minimax-cn",
            KnownProvider::Huggingface => "huggingface",
            KnownProvider::Opencode => "opencode",
            KnownProvider::OpencodeGo => "opencode-go",
            KnownProvider::KimiCoding => "kimi-coding",
        }
    }
}

impl std::fmt::Display for KnownProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCost {
    pub input: f64,
    pub output: f64,
    #[serde(default)]
    pub cache_read: f64,
    #[serde(default)]
    pub cache_write: f64,
}

impl ModelCost {
    pub fn new(input: f64, output: f64) -> Self {
        Self {
            input,
            output,
            cache_read: 0.0,
            cache_write: 0.0,
        }
    }

    pub fn with_cache_read(mut self, cache_read: f64) -> Self {
        self.cache_read = cache_read;
        self
    }

    pub fn with_cache_write(mut self, cache_write: f64) -> Self {
        self.cache_write = cache_write;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub api: Api,
    pub provider: KnownProvider,
    pub base_url: String,
    pub reasoning: bool,
    pub input: Vec<ModelInput>,
    pub cost: ModelCost,
    pub context_window: u32,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelInput {
    Text,
    Image,
}

impl Model {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        api: Api,
        provider: KnownProvider,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            api,
            provider,
            base_url: base_url.into(),
            reasoning: false,
            input: vec![ModelInput::Text],
            cost: ModelCost::new(0.0, 0.0),
            context_window: 0,
            max_tokens: 4096,
            headers: None,
        }
    }

    pub fn with_reasoning(mut self, reasoning: bool) -> Self {
        self.reasoning = reasoning;
        self
    }

    pub fn with_input(mut self, input: Vec<ModelInput>) -> Self {
        self.input = input;
        self
    }

    pub fn with_cost(mut self, cost: ModelCost) -> Self {
        self.cost = cost;
        self
    }

    pub fn with_context_window(mut self, context_window: u32) -> Self {
        self.context_window = context_window;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_headers(mut self, headers: std::collections::HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }
}
