// src/providers/groq.rs
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::{Client, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use std::env;
use std::pin::Pin;
use url::Url;

use crate::error::{ReplError, ReplResult};
use crate::providers::LlmProvider;

// --- Structs for Groq API (OpenAI Compatible) ---
// Request Structures (These should be correct)
#[derive(Serialize, Debug)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    // Add other optional parameters here if needed: temperature, top_p, etc.
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChatMessage {
    role: Role,
    content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Role {
    System,
    User,
    Assistant,
}

// Streaming Response Structures (These should be generally correct for OpenAI format)
#[derive(Deserialize, Debug)]
struct ChatCompletionChunk {
    // id: String, // Optional ID for the chunk
    // object: String, // e.g., "chat.completion.chunk"
    // created: u64, // Timestamp
    // model: String, // Model used
    choices: Vec<DeltaChoice>,
    // system_fingerprint: Option<String>, // Optional fingerprint
}

#[derive(Deserialize, Debug)]
struct DeltaChoice {
    index: u32,
    delta: Delta,
    logprobs: Option<serde_json::Value>, // Handle potential streaming logprobs if ever added
    finish_reason: Option<String>, // e.g., "stop", "length"
}

#[derive(Deserialize, Debug)]
struct Delta {
    // role: Option<Role>, // Might appear in the first delta
    content: Option<String>,
}

// Non-Streaming Response Structures (Meticulously matched to example)
#[derive(Deserialize, Debug)]
struct ChatCompletionResponse {
    id: String,
    object: String, // e.g., "chat.completion"
    created: u64,   // Timestamp
    model: String,
    choices: Vec<ResponseMessageChoice>,
    usage: UsageStats,
    system_fingerprint: Option<String>, // Can be optional
    x_groq: Option<XGroq>,              // Vendor specific, make optional
}

#[derive(Deserialize, Debug)]
struct ResponseMessageChoice {
    index: u32,
    message: ResponseMessage,
    logprobs: Option<serde_json::Value>, // Use Value for flexibility (null or object)
    finish_reason: String,               // e.g., "stop"
}

#[derive(Deserialize, Debug)]
struct ResponseMessage {
    role: Role,
    content: String,
}

#[derive(Deserialize, Debug)]
struct UsageStats {
    queue_time: Option<f64>, // Make queue time optional, might not always be present
    prompt_tokens: u64,
    prompt_time: Option<f64>, // Make prompt time optional
    completion_tokens: u64,
    completion_time: f64,
    total_tokens: u64,
    total_time: f64,
}

#[derive(Deserialize, Debug)]
struct XGroq { // Structure for the vendor-specific x_groq field
    id: Option<String>, // Make internal fields optional too
}

// Model Listing Structures (These should be correct)
#[derive(Deserialize, Debug)]
struct GroqModel {
    id: String,
    object: String,
    created: u64,
    owned_by: String,
    active: bool,
    context_window: u32,
}

#[derive(Deserialize, Debug)]
struct GroqModelList {
    object: String,
    data: Vec<GroqModel>,
}


const GROQ_API_BASE_URL: &str = "https://api.groq.com/openai/v1/";

#[derive(Debug, Clone)]
pub struct GroqProvider {
    client: Client,
    api_key: Option<String>, // API Key is now optional
    base_url: Url,
}

impl GroqProvider {
    /// Creates a new Groq provider instance.
    /// Attempts to load the API key from GROQ_API_KEY env var.
    /// Prints an INFO message if the key is missing/empty but still creates the provider.
    pub fn new() -> Self {
        let api_key = env::var("GROQ_API_KEY").ok().filter(|k| !k.is_empty());

        if api_key.is_none() {
            println!(
                "INFO: GROQ_API_KEY not set or empty. Groq provider initialized but unusable until key is set and app is restarted."
            );
        }

        // Static URL parsing is unlikely to fail, use expect for simplicity
        let base_url = Url::parse(GROQ_API_BASE_URL)
            .expect("Static Groq base URL should be valid");

        Self {
            client: Client::new(),
            api_key, // Store None if key wasn't found/valid
            base_url,
        }
    }

    /// Helper to build a full URL for a given API endpoint.
    fn build_url(&self, endpoint: &str) -> Result<Url, ReplError> {
        self.base_url
            .join(endpoint)
            .map_err(|e| ReplError::Provider(format!("Failed to build Groq URL: {}", e)))
    }

    /// Helper to add the Authorization header (Bearer token).
    fn add_auth(&self, builder: RequestBuilder, api_key: &str) -> RequestBuilder {
        builder.bearer_auth(api_key)
    }

    /// Helper to construct a standardized error from an API response.
    async fn handle_api_error(response: Response) -> ReplError {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        ReplError::Provider(format!(
            "Groq API error: {} - {}",
            status, body
        ))
    }

    /// Centralized check for API key availability before making a call.
    fn get_api_key(&self) -> ReplResult<&String> {
        self.api_key.as_ref().ok_or_else(|| {
            ReplError::Provider(
                "Groq API key is missing. Set the GROQ_API_KEY environment variable and restart."
                    .to_string(),
            )
        })
    }
}

#[async_trait]
impl LlmProvider for GroqProvider {
    fn get_name(&self) -> &str {
        "groq"
    }
    async fn check_readiness(&self) -> ReplResult<()> {
               self.get_api_key()?; // Call the existing key check helper
               Ok(()) // If get_api_key didn't return Err, we are ready
        }

    fn clone_box(&self) -> Box<dyn LlmProvider> {
        Box::new(self.clone())
    }

    async fn get_models(&self) -> ReplResult<Vec<String>> {
        let api_key = self.get_api_key()?; // Check for API key first
        let url = self.build_url("models")?;
        let response = self.add_auth(self.client.get(url), api_key).send().await?;

        if !response.status().is_success() {
            return Err(Self::handle_api_error(response).await);
        }

        let model_list_response = response.json::<GroqModelList>().await?;
        let model_names = model_list_response.data.into_iter()
            .map(|m| m.id)
            .collect();
        Ok(model_names)
    }

// --- Inside impl LlmProvider for GroqProvider ---

    // Keep get_name, clone_box, get_models, query as they are

    async fn query_stream(
        &self,
        model: &str,
        prompt: &str,
    ) -> ReplResult<Option<Pin<Box<dyn Stream<Item = ReplResult<String>> + Send>>>> {
        let api_key = self.get_api_key()?;
        let url = self.build_url("chat/completions")?;
        let messages = vec![ChatMessage { role: Role::User, content: prompt.to_string() }];
        let body = ChatCompletionRequest { model: model.to_string(), messages, stream: true };

        let response = self.add_auth(self.client.post(url).json(&body), api_key).send().await?;

        if !response.status().is_success() {
            return Err(Self::handle_api_error(response).await);
        }

        let byte_stream = response.bytes_stream();

        // Use a state machine approach to reassemble potentially fragmented SSE messages
        let stream = futures::stream::unfold(
            (byte_stream, String::new()), // State: (underlying stream, leftover buffer from previous chunk)
            |(mut stream, mut buffer)| async move {
                loop {
                    // Check buffer first for complete messages
                    if let Some(end_idx) = buffer.find("\n\n") {
                        let message = buffer.drain(..end_idx + 2).collect::<String>(); // Consume message + delimiters
                        if let Some(content) = process_sse_message(&message) {
                            // Yield content if message parsed successfully
                            return Some((Ok(content), (stream, buffer)));
                        }
                        // If processing failed or yielded no content, continue loop to get more data or check buffer again
                        continue;
                    }

                    // Buffer doesn't have a complete message, read more from the network stream
                    match stream.next().await {
                        Some(Ok(bytes)) => {
                            // Append new data to buffer
                            match String::from_utf8(bytes.to_vec()) {
                                Ok(text) => buffer.push_str(&text),
                                Err(e) => {
                                    // UTF-8 error in chunk, yield error and stop
                                    let err = ReplError::Provider(format!("Stream chunk not valid UTF-8: {}", e));
                                    return Some((Err(err), (stream, buffer)));
                                }
                            };
                            // Loop back to check buffer again with new data
                        }
                        Some(Err(e)) => {
                            // Network error reading stream, yield error and stop
                            let err = ReplError::Request(e);
                            return Some((Err(err), (stream, buffer)));
                        }
                        None => {
                            // End of network stream
                            // Process any remaining data in the buffer
                            if !buffer.is_empty() {
                                if let Some(content) = process_sse_message(&buffer) {
                                    buffer.clear(); // Clear buffer after processing
                                    return Some((Ok(content), (stream, buffer)));
                                } else {
                                     // Remaining buffer couldn't be processed or was empty content
                                    return None; // End the stream
                                }
                            } else {
                                // Buffer is empty and stream ended
                                return None; // End the stream
                            }
                        }
                    }
                }
            },
        )
        .filter_map(|res| async move { // Keep filtering empty strings and propagate errors
             match res {
                 Ok(s) if !s.is_empty() => Some(Ok(s)),
                 Ok(_) => None,
                 Err(e) => Some(Err(e)),
             }
         });

        Ok(Some(Box::pin(stream)))
    } // <-- End of query_stream function

// --- Keep the rest of the impl block ---
} // <-- End of impl LlmProvider

/// Helper function to process a potential complete SSE message block
/// Returns Some(content) if parsing is successful and yields content,
/// None otherwise (e.g., not a data message, empty content, parse error).
fn process_sse_message(message_block: &str) -> Option<String> {
    let mut content_acc = String::new();
    for line in message_block.lines() {
        if line.starts_with("data:") {
            let data = line[5..].trim();
            if data == "[DONE]" {
                // Although we might get [DONE] here, we typically rely on stream ending.
                // Return None as this marker yields no displayable content.
                return None;
            }
            if data.is_empty() {
                continue;
            }

            match serde_json::from_str::<ChatCompletionChunk>(data) {
                Ok(parsed_chunk) => {
                    for choice in parsed_chunk.choices {
                        if let Some(content) = choice.delta.content {
                            content_acc.push_str(&content);
                        }
                    }
                }
                Err(e) => {
                    // Log parsing error for this specific data line but continue processing block
                    eprintln!(
                        "Failed to parse stream data line JSON. Error: {}. Data: '{}'",
                        e, data
                    );
                    // Decide whether to halt or continue. Let's return None for this message block.
                    return None;
                }
            }
        }
        // Ignore comment lines (:) or other non-data lines within the block
    }

    if content_acc.is_empty() { None } else { Some(content_acc) }
}