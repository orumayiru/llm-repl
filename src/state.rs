// src/state.rs
use crate::{
    commands::CommandRegistry, // Only need CommandRegistry
    error::{ReplError, ReplResult},
    providers::{LlmProvider, ProviderRegistry},
};
use serde::{Deserialize, Serialize}; // Import Serde traits
use std::sync::Arc;
use tokio::sync::Mutex;

// --- History Structures ---
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryContentType {
    LlmResponse { model: String },
    CommandResult { command: String },
    ShellOutput { command: String },
    UserQuery,
    Error { source: String },
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub entry_type: HistoryContentType,
    pub content: String,
}
// --- End History Structures ---

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MarkdownMode {
    AppendFormatted,
    LiveStreaming,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RenderTheme {
    Default,
    Nord,
    Gruvbox,
    Grayscale,
}

// AppState holds the application's shared state.
pub struct AppState {
    provider_registry: ProviderRegistry,
    // CommandRegistry is wrapped in Arc for cheap cloning and sharing.
    command_registry: Arc<CommandRegistry>,
    current_provider: Arc<Mutex<String>>,
    current_model: Arc<Mutex<String>>,
    current_markdown_mode: Arc<Mutex<MarkdownMode>>,
    current_theme: Arc<Mutex<RenderTheme>>,
    output_history: Arc<Mutex<Vec<HistoryEntry>>>,
}

// Manual Clone implementation because CommandRegistry is not Clone by default.
impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            provider_registry: self.provider_registry.clone(), // ProviderRegistry derives Clone
            command_registry: Arc::clone(&self.command_registry), // Clone the Arc pointer
            current_provider: Arc::clone(&self.current_provider),
            current_model: Arc::clone(&self.current_model),
            current_markdown_mode: Arc::clone(&self.current_markdown_mode),
            current_theme: Arc::clone(&self.current_theme),
            output_history: Arc::clone(&self.output_history),
        }
    }
}


impl AppState {
    /// Creates the application state, including initializing and registering commands.
    pub fn new() -> Self {
        // Step 1: Initialize basic components and state Arcs
        let provider_registry = ProviderRegistry::new();
        let initial_provider = "ollama";
        let initial_model = "llama3:latest"; // Ensure this is a valid default

        let provider_registry_arc = provider_registry; // Assuming ProviderRegistry is Clone
        let current_provider_arc = Arc::new(Mutex::new(initial_provider.to_string()));
        let current_model_arc = Arc::new(Mutex::new(initial_model.to_string()));
        let current_markdown_mode_arc = Arc::new(Mutex::new(MarkdownMode::AppendFormatted));
        let current_theme_arc = Arc::new(Mutex::new(RenderTheme::Nord));
        let output_history_arc = Arc::new(Mutex::new(Vec::new()));

        // Step 2: Create a preliminary AppState instance.
        // This instance is needed to pass state to CommandRegistry::new().
        // It gets a temporary, empty CommandRegistry Arc initially.
        let preliminary_state = AppState {
            provider_registry: provider_registry_arc.clone(),
            command_registry: Arc::new(CommandRegistry::new_empty()), // Use empty constructor
            current_provider: current_provider_arc.clone(),
            current_model: current_model_arc.clone(),
            current_markdown_mode: current_markdown_mode_arc.clone(),
            current_theme: current_theme_arc.clone(),
            output_history: output_history_arc.clone(),
        };

        // Step 3: Create the *actual* fully populated CommandRegistry, passing the preliminary state clone.
        // CommandRegistry::new internally uses this state clone to initialize individual commands.
        let final_command_registry = CommandRegistry::new(preliminary_state.clone());

        // Step 4: Construct the final AppState using the final components, including the real registry.
        AppState {
            provider_registry: provider_registry_arc,
            command_registry: Arc::new(final_command_registry), // Store the real registry in Arc
            current_provider: current_provider_arc,
            current_model: current_model_arc,
            current_markdown_mode: current_markdown_mode_arc,
            current_theme: current_theme_arc,
            output_history: output_history_arc,
        }
    }

    // --- Getters and Setters ---
    pub async fn get_provider_name(&self) -> String { self.current_provider.lock().await.clone() }
    pub async fn set_model(&self, model: &str) -> ReplResult<()> { let mut current_model = self.current_model.lock().await; *current_model = model.trim().to_string(); Ok(()) }
    pub async fn get_model(&self) -> String { self.current_model.lock().await.clone() }
    pub async fn get_current_provider(&self) -> Option<Box<dyn LlmProvider>> { let provider_name = self.get_provider_name().await; self.provider_registry.get_provider(&provider_name).map(|p| p.clone_box()) }
    pub fn get_provider_by_name(&self, name: &str) -> Option<Box<dyn LlmProvider>> { self.provider_registry.get_provider(name).map(|p| p.clone_box()) }
    pub fn list_providers(&self) -> Vec<String> { self.provider_registry.list_providers().into_iter().map(String::from).collect() }
    pub async fn set_provider(&self, provider_name: &str) -> ReplResult<()> {
        let provider_name_lower = provider_name.trim().to_lowercase();
        let provider = match self.provider_registry.get_provider(&provider_name_lower) { Some(p) => p, None => return Err(ReplError::UnknownProvider(provider_name_lower)), };
        provider.check_readiness().await.map_err(|e| { ReplError::Provider(format!("Provider '{}' is not ready: {}", provider_name_lower, e)) })?;
        let mut current_provider_guard = self.current_provider.lock().await;
        if *current_provider_guard != provider_name_lower {
            *current_provider_guard = provider_name_lower.clone(); drop(current_provider_guard); println!("Provider set to: {}", provider_name_lower);
            match provider.get_models().await {
                Ok(models) if !models.is_empty() => { if self.set_model(&models[0]).await.is_ok() { println!("Automatically selected model: {}", &models[0]); } else { eprintln!("WARN: Failed to update model state after provider change."); } }
                Ok(_) => { println!("WARN: Provider '{}' reported no available models. Model unchanged.", provider_name_lower); }
                Err(e) => { eprintln!("WARN: Could not fetch models for provider '{}': {}. Model unchanged.", provider_name_lower, e); }
            }
        } else { println!("Provider already set to: {}", provider_name_lower); return Ok(()); }
        Ok(())
    }
    pub async fn list_models(&self) -> ReplResult<Vec<String>> {
         if let Some(provider) = self.get_current_provider().await { provider.get_models().await }
         else { let provider_name = self.get_provider_name().await; Err(ReplError::Provider(format!("Current provider '{}' not found or unavailable.", provider_name))) }
    }
    pub async fn get_markdown_mode(&self) -> MarkdownMode { *self.current_markdown_mode.lock().await }
    pub async fn set_markdown_mode(&self, mode: MarkdownMode) { let mut current_mode_guard = self.current_markdown_mode.lock().await; *current_mode_guard = mode; }
    pub async fn get_theme(&self) -> RenderTheme { *self.current_theme.lock().await }
    pub async fn set_theme(&self, theme: RenderTheme) { let mut current_theme_guard = self.current_theme.lock().await; *current_theme_guard = theme; }
    pub async fn add_history_entry(&self, entry: HistoryEntry) { let mut history = self.output_history.lock().await; history.push(entry); }
    pub async fn get_history(&self) -> Vec<HistoryEntry> { self.output_history.lock().await.clone() }

    /// Provides read-only access to the command registry Arc.
    pub fn command_registry(&self) -> Arc<CommandRegistry> {
        Arc::clone(&self.command_registry)
    }
}