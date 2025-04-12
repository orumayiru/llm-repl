use std::pin::Pin;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;


use crate::error::{ReplError, ReplResult};
use super::LlmProvider;

#[derive(Serialize, Deserialize, Debug)]
struct OllamaResponse {
    response: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OllamaModel {
    name: String,
    
}

#[derive(Serialize, Deserialize, Debug)]
struct OllamaListResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseChunk {
    response: Option<String>, 
    done: bool,
    model: Option<String>,
    created_at: Option<String>,
}


impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new("http://localhost:11434").expect("Failed to create default Ollama provider")
    }
}

impl OllamaProvider {
    /// Create new Ollama provider with custom URL
    pub fn new(base_url: &str) -> Result<Self, ReplError> {
        let base_url = Url::parse(base_url)
            .map_err(|e| ReplError::Provider(format!("Invalid Ollama URL: {}", e)))?;

        Ok(Self {
            client: Client::new(),
            base_url,
        })
    }

    fn build_url(&self, endpoint: &str) -> Result<Url, ReplError> {
        self.base_url.join(endpoint)
            .map_err(|e| ReplError::Provider(format!("Failed to build URL: {}", e)))
    }

    async fn fetch_models_from_api(&self) -> ReplResult<Vec<String>> {
        let url = self.build_url("api/tags")?;
        let response: Response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| ReplError::Provider(format!("Failed to send request to Ollama: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            return Err(ReplError::Provider(format!("Ollama API returned an error: {} - {}", status, error_body)));
        }

        let ollama_response = response.json::<OllamaListResponse>().await.map_err(|e| ReplError::Provider(format!("Failed to parse Ollama API response: {}", e)))?;

        let model_names = ollama_response.models.into_iter().map(|model| model.name).collect();
        Ok(model_names)
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn query(&self, model: &str, prompt: &str) -> ReplResult<String> {
        let url = self.build_url("api/generate")?;
        let body = json!({
            "model": model,
            "prompt": prompt,
            "stream": true
        });

        let response = self.client
            .post(url)
            .json(&body)
            .send()
            .await?;

        let status = response.status(); // Get the status code here

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            return Err(ReplError::Provider(format!("Ollama API returned an error: {} - {}", status, error_body)));
        }

        let mut stream = response.bytes_stream();
        let mut full_response = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| ReplError::Provider(format!("Error reading stream chunk: {}", e)))?;
            if !chunk.is_empty() {
                match serde_json::from_slice::<OllamaResponseChunk>(&chunk) {
                    Ok(response_part) => {
                        if let Some(response) = response_part.response {
                            full_response.push_str(&response);
                        }
                        if response_part.done {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error deserializing stream chunk: {}", e);
                        // Optionally handle the error more specifically or break the stream
                    }
                }
            }
        }

        Ok(full_response)
    }
    async fn query_stream(
        &self,
        model: &str,
        prompt: &str,
    ) -> ReplResult<Option<Pin<Box<dyn Stream<Item = ReplResult<String>> + Send>>>> {
        let url = self.build_url("api/generate")?;
        let body = json!({
            "model": model,
            "prompt": prompt,
            "stream": true
        });

        let response = self.client
            .post(url)
            .json(&body)
            .send()
            .await?;

        let stream = response
            .bytes_stream()
            .map(|chunk| match chunk {
                Ok(bytes) => {
                    let s = String::from_utf8(bytes.to_vec())
                        .map_err(|e| ReplError::Json(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?;
                    let chunk: OllamaResponseChunk = serde_json::from_str(&s)
                        .map_err(|e| ReplError::Json(e))?;
                    Ok(chunk.response.unwrap_or_default())
                }
                Err(e) => Err(ReplError::Request(e)),
            });

        Ok(Some(Box::pin(stream)))
    }
    async fn get_models(&self) -> ReplResult<Vec<String>> {
        self.fetch_models_from_api().await
    }

    fn get_name(&self) -> &str {
        "ollama"
    }
    fn clone_box(&self) -> Box<dyn LlmProvider> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    client: Client,
    base_url: Url,
}