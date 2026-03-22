use crate::models::Api;
use crate::types::Context;
use crate::stream::StreamResponse;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("API request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    
    #[error("streaming failed: {0}")]
    StreamFailed(String),
    
    #[error("parse error: {0}")]
    ParseError(String),
    
    #[error("auth error: {0}")]
    AuthError(String),
    
    #[error("provider error: {0}")]
    Provider(String),
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn api(&self) -> Api;
    fn model_id(&self) -> &str;
    
    async fn stream(&self, context: &Context) -> Result<StreamResponse>;
    
    async fn complete(&self, context: &Context) -> Result<crate::types::AssistantMessage> {
        let mut stream = self.stream(context).await?;
        
        let mut last_event = None;
        
        while let Some(result) = futures::StreamExt::next(&mut stream).await {
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
}

#[derive(Debug, Clone)]
pub struct Response {
    pub content: String,
}
