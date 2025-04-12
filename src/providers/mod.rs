// src/providers/mod.rs
use std::collections::HashMap;
use std::pin::Pin;
use async_trait::async_trait;
use futures::Stream;
use crate::error::ReplResult;
use crate::error::ReplError;

pub mod ollama;
pub mod groq;
pub mod gemini;
/// Core provider trait for LLM interactions
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Standard query that returns complete response
    async fn query(&self, _model: &str, _prompt: &str) -> ReplResult<String> {
        unimplemented!()
    }
    
    async fn query_stream(
        &self,
        _model: &str,
        _prompt: &str,
    ) -> ReplResult<Option<Pin<Box<dyn Stream<Item = ReplResult<String>> + Send>>>> {
        unimplemented!()
    }
    
    async fn get_models(&self) -> ReplResult<Vec<String>> {
                 Err(ReplError::Provider(format!(
                    "get_models not implemented for provider {}",
                    self.get_name()
                )))
             }
    async fn check_readiness(&self) -> ReplResult<()> {
            Ok(()) // Default implementation: provider is always ready
    }
    
    fn get_name(&self) -> &str {
        unimplemented!()
    }
    
    fn clone_box(&self) -> Box<dyn LlmProvider> {
        unimplemented!()
    }
}

/// Implement Clone for boxed providers
impl Clone for Box<dyn LlmProvider> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Provider registry maintains all available providers
#[derive(Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn LlmProvider>>,
}

impl ProviderRegistry {
    /// Create new registry with default providers
    pub fn new() -> Self {
        let mut registry = ProviderRegistry {
            providers: HashMap::new(),
        };
        // Register default providers
        registry.register(Box::new(ollama::OllamaProvider::default()));
        // Attempt to register Groq if API key is available
        registry.register(Box::new(groq::GroqProvider::new()));
        registry.register(Box::new(gemini::GeminiProvider::new()));
        registry
    }
    
    /// Register a new provider
    pub fn register(&mut self, provider: Box<dyn LlmProvider>) {
        self.providers.insert(provider.get_name().to_string(), provider);
    }
    
    /// Get provider by name
    pub fn get_provider(&self, name: &str) -> Option<&dyn LlmProvider> {
        self.providers.get(name).map(|p| &**p)
    }
    
    /// List all available provider names
    pub fn list_providers(&self) -> Vec<&str> {
        self.providers.keys().map(|k| k.as_str()).collect()
    }
    
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

