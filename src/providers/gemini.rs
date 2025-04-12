// src/providers/gemini.rs
use async_trait::async_trait;
use bytes::BytesMut;
use futures::{Stream, StreamExt};
use reqwest::{Client, Response};
use serde::{de::DeserializeSeed, Deserialize, Serialize};
use serde_json::StreamDeserializer;
use std::env;
use std::pin::Pin;
use url::Url;

use crate::error::{ReplError, ReplResult};
use crate::providers::LlmProvider;

// --- Gemini API Specific Structs ---
#[derive(Serialize, Debug)]
struct GeminiGenerateContentRequest { contents: Vec<Content> }

// --- CORRECTED Content Struct ---
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Content {
    // Role might also be optional in some edge cases or final chunks, let's keep required for now
    role: String,
    // Parts is definitely optional, especially in the final chunk
    parts: Option<Vec<Part>>, // <-- Make Option<>
}
// --- End CORRECTED Content Struct ---

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Part { text: String }

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerateContentResponse { candidates: Option<Vec<Candidate>> }

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct GeminiStreamChunk { candidates: Option<Vec<Candidate>> }

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    // Content itself can be optional (e.g., if only finishReason is sent)
    content: Option<Content>, // <-- Keep as Option<>
    finish_reason: Option<String>,
    safety_ratings: Option<Vec<SafetyRating>>,
    #[allow(dead_code)] token_count: Option<u32>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct SafetyRating { #[allow(dead_code)] category: String, probability: String, }

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GeminiModel { name: String, display_name: Option<String>, description: Option<String>, supported_generation_methods: Option<Vec<String>>, }

#[derive(Deserialize, Debug)]
struct GeminiModelList { models: Vec<GeminiModel> }

#[derive(Deserialize, Debug)]
struct GoogleApiErrorResponse { error: GoogleApiError }

#[derive(Deserialize, Debug)]
struct GoogleApiError { code: u16, message: String, status: String }
// --- End Structs ---

const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/";

#[derive(Debug, Clone)]
pub struct GeminiProvider {
    client: Client,
    api_key: Option<String>,
    base_url: Url,
}

// --- impl GeminiProvider (Helpers remain the same) ---
impl GeminiProvider {
    pub fn new() -> Self {
        let api_key_opt = env::var("GOOGLE_API_KEY").ok().filter(|k| !k.is_empty());
        if api_key_opt.is_none() { println!("INFO: GOOGLE_API_KEY env var not set or empty. Gemini provider will be unavailable until set and app restarted."); }
        let base_url = Url::parse(GEMINI_API_BASE_URL).expect("Static Gemini base URL should be valid");
        Self { client: Client::new(), api_key: api_key_opt, base_url }
    }
    fn build_action_url(&self, model_id: &str, action: &str, api_key: &str) -> ReplResult<Url> {
        let clean_model_id = model_id.strip_prefix("models/").unwrap_or(model_id);
        let path = format!("models/{}:{}", clean_model_id, action);
        let mut url = self.base_url.join(&path).map_err(|e| ReplError::Provider(format!("Failed to build Gemini URL path: {}", e)))?;
        url.query_pairs_mut().append_pair("key", api_key); Ok(url)
    }
    fn build_list_models_url(&self, api_key: &str) -> ReplResult<Url> {
        let path = "models";
        let mut url = self.base_url.join(path).map_err(|e| ReplError::Provider(format!("Failed to build Gemini URL path: {}", e)))?;
        url.query_pairs_mut().append_pair("key", api_key); Ok(url)
    }
    async fn handle_api_error(response: Response) -> ReplError {
        let status = response.status();
        match response.bytes().await {
            Ok(bytes) => {
                match serde_json::from_slice::<GoogleApiErrorResponse>(&bytes) {
                    Ok(err_resp) => ReplError::Provider(format!("Gemini API error: {} - {} (Status: {}, Code: {})", status, err_resp.error.message, err_resp.error.status, err_resp.error.code)),
                    Err(_) => { let body = String::from_utf8_lossy(&bytes); ReplError::Provider(format!("Gemini API error: {} - {} (Raw Body: {})", status, status.canonical_reason().unwrap_or("Unknown Status"), body.trim())) }
                }
            }
            Err(e) => ReplError::Provider(format!("Gemini API error: {} - Failed to read error body: {}", status, e)),
        }
    }
    fn get_api_key(&self) -> ReplResult<&String> {
        self.api_key.as_ref().ok_or_else(|| ReplError::Provider("Google API key is missing. Set GOOGLE_API_KEY environment variable and restart.".to_string()))
    }
    // Corrected format_single_prompt for the modified Content struct
    fn format_single_prompt(&self, prompt: &str) -> Vec<Content> {
        vec![Content {
            role: "user".to_string(),
            // Ensure parts is Some when constructing the request
            parts: Some(vec![Part { text: prompt.to_string() }]),
        }]
    }
}


#[async_trait]
impl LlmProvider for GeminiProvider {
    fn get_name(&self) -> &str { "gemini" }
    async fn check_readiness(&self) -> ReplResult<()> { self.get_api_key()?; Ok(()) }
    fn clone_box(&self) -> Box<dyn LlmProvider> { Box::new(self.clone()) }
    async fn get_models(&self) -> ReplResult<Vec<String>> { /* ... No changes ... */
        let api_key = self.get_api_key()?;
        let url = self.build_list_models_url(api_key)?;
        let response = self.client.get(url).send().await.map_err(ReplError::Request)?;
        if !response.status().is_success() { return Err(Self::handle_api_error(response).await); }
        let response_bytes = response.bytes().await.map_err(ReplError::Request)?;
        match serde_json::from_slice::<GeminiModelList>(&response_bytes) {
            Ok(model_list_response) => { let model_names = model_list_response.models.into_iter().filter(|m| m.supported_generation_methods.as_ref().map_or(false, |methods| methods.contains(&"generateContent".to_string()) || methods.contains(&"streamGenerateContent".to_string()))).map(|m| m.name).collect(); Ok(model_names) }
            Err(e) => { let body_text = String::from_utf8_lossy(&response_bytes); eprintln!("--- Gemini Model List Raw Response ---"); eprintln!("{}", body_text); eprintln!("------------------------------------"); Err(ReplError::Json(e)) }
        }
    }

    // --- Corrected query to handle optional parts ---
    async fn query(&self, model: &str, prompt: &str) -> ReplResult<String> {
        let api_key = self.get_api_key()?;
        let url = self.build_action_url(model, "generateContent", api_key)?;
        let contents = self.format_single_prompt(prompt);
        let body = GeminiGenerateContentRequest { contents };
        let response = self.client.post(url).json(&body).send().await.map_err(ReplError::Request)?;
        if !response.status().is_success() { return Err(Self::handle_api_error(response).await); }
        let response_body = response.json::<GeminiGenerateContentResponse>().await.map_err(ReplError::Request)?;

        // --- Adjusted text extraction ---
        let text = response_body.candidates
            .and_then(|cands| cands.into_iter().next())
            .and_then(|cand| cand.content)
            .and_then(|cont| cont.parts) // cont.parts is now Option<Vec<Part>>
            .and_then(|parts_vec| parts_vec.into_iter().next()) // Get first part from the Vec
            .map(|part| part.text);

        match text {
            Some(t) => Ok(t),
            None => Err(ReplError::Provider("Gemini non-streaming response missing expected text content.".to_string())),
        }
    }

    // --- Corrected query_stream to handle optional parts ---
    async fn query_stream(
        &self,
        model: &str,
        prompt: &str,
    ) -> ReplResult<Option<Pin<Box<dyn Stream<Item = ReplResult<String>> + Send>>>> {
        let api_key = self.get_api_key()?;
        let url = self.build_action_url(model, "streamGenerateContent", api_key)?;
        let contents = self.format_single_prompt(prompt); // Uses corrected format_single_prompt
        let body = GeminiGenerateContentRequest { contents };
        let response = self.client.post(url).json(&body).send().await.map_err(ReplError::Request)?;
        if !response.status().is_success() { return Err(Self::handle_api_error(response).await); }

        let byte_stream = response.bytes_stream();
        let stream = futures::stream::unfold(
            (byte_stream, BytesMut::new()),
            |(mut stream, mut buffer)| async move {
                loop {
                    let mut stream_deserializer = StreamDeserializer::<_, Vec<GeminiStreamChunk>>::new(serde_json::de::IoRead::new(buffer.as_ref()));
                    match stream_deserializer.next() {
                        Some(Ok(chunk_vec)) => {
                            let consumed = stream_deserializer.byte_offset();
                            let mut combined_text_for_event = String::new();
                            for chunk in chunk_vec {
                                if let Some(candidates) = chunk.candidates {
                                    for candidate in candidates {
                                        if let Some(reason) = &candidate.finish_reason { if reason.to_uppercase() == "SAFETY" { eprintln!("\n[WARN: Potential safety block/filter by Gemini]"); } }
                                        // --- Handle optional parts here ---
                                        if let Some(content) = &candidate.content {
                                            if let Some(parts) = &content.parts { // Check if parts exists
                                                for part in parts {
                                                    combined_text_for_event.push_str(&part.text);
                                                }
                                            }
                                        }
                                        // --- End optional parts handling ---
                                    }
                                }
                            }
                            let _ = buffer.split_to(consumed);
                            if !combined_text_for_event.is_empty() {
                                return Some((Ok(combined_text_for_event), (stream, buffer)));
                            } else { continue; }
                        }
                        Some(Err(e)) if e.is_eof() => { break; }
                        Some(Err(e)) => {
                            eprintln!("ERROR: Gemini stream JSON parsing error: {}", e); eprintln!("Buffer content causing error: {:?}", String::from_utf8_lossy(&buffer));
                            let error = ReplError::Json(e); buffer.clear(); return Some((Err(error), (stream, buffer)));
                        }
                        None => { if buffer.is_empty() { break; } else { eprintln!("WARN: StreamDeserializer<Vec<Chunk>> yielded None despite non-empty buffer: {:?}", String::from_utf8_lossy(&buffer)); buffer.clear(); break; } }
                    }
                } // End inner loop
                match stream.next().await {
                    Some(Ok(bytes_chunk)) => { buffer.extend_from_slice(&bytes_chunk); Some((Ok(String::new()), (stream, buffer))) }
                    Some(Err(e)) => { let error = ReplError::Request(e); return Some((Err(error), (stream, buffer))); }
                    None => { if !buffer.is_empty() { eprintln!("WARN: Gemini stream ended with final unprocessed buffer: {:?}", String::from_utf8_lossy(&buffer)); } return None; }
                }
            },
        )
        .filter_map(|res| async move { match res { Ok(s) if !s.is_empty() => Some(Ok(s)), Ok(_) => None, Err(e) => Some(Err(e)), } });

        Ok(Some(Box::pin(stream)))
    }
} // End impl LlmProvider for GeminiProvider