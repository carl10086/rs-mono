use crate::models::{Api, Model};
use crate::types::Context;
use crate::stream::StreamResponse;
use crate::provider::{Provider, ProviderError};
use anyhow::Result;
use futures::StreamExt;
use std::sync::Mutex;

static REGISTRY: std::sync::OnceLock<Mutex<crate::api_registry::ApiProviderRegistry>> = std::sync::OnceLock::new();

pub fn get_registry() -> &'static Mutex<crate::api_registry::ApiProviderRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(crate::api_registry::ApiProviderRegistry::new()))
}

pub fn register_provider<P: Provider + 'static>(provider: P) {
    get_registry().lock().unwrap().register(provider);
}

pub fn get_provider(_api: &Api) -> Option<std::sync::MutexGuard<'static, crate::api_registry::ApiProviderRegistry>> {
    Some(get_registry().lock().unwrap())
}

pub async fn stream(
    model: &Model,
    context: &Context,
) -> Result<StreamResponse> {
    let registry = get_registry().lock().unwrap();
    let provider = registry.get(&model.api)
        .ok_or_else(|| anyhow::anyhow!("No provider registered for api: {}", model.api))?;
    
    provider.stream(context).await
}

pub async fn complete(model: &Model, context: &Context) -> Result<crate::types::AssistantMessage> {
    let mut stream = stream(model, context).await?;
    
    let mut last_event = None;
    
    while let Some(result) = stream.next().await {
        let event = result.map_err(|e| ProviderError::StreamFailed(e.to_string()))?;
        last_event = Some(event);
    }
    
    if let Some(crate::StreamEvent::Done { message, .. }) = last_event {
        Ok(message)
    } else if let Some(crate::StreamEvent::Error { error, .. }) = last_event {
        Err(anyhow::anyhow!("Provider error: {}", error))
    } else {
        Err(anyhow::anyhow!("Stream ended without message"))
    }
}

pub async fn stream_simple(
    model: &Model,
    context: &mut Context,
    reasoning: Option<crate::types::ThinkingLevel>,
) -> Result<StreamResponse> {
    if let Some(level) = reasoning {
        context.thinking = Some(level);
    }
    
    stream(model, context).await
}

pub async fn complete_simple(
    model: &Model,
    context: &mut Context,
    reasoning: Option<crate::types::ThinkingLevel>,
) -> Result<crate::types::AssistantMessage> {
    if let Some(level) = reasoning {
        context.thinking = Some(level);
    }
    
    complete(model, context).await
}
