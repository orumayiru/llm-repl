// src/commands/provider.rs
use async_trait::async_trait;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};

use crate::{
    commands::Command,
    error::{ReplError, ReplResult},
    state::AppState,
};

#[derive(Clone)]
pub struct ProviderCommand {
    state: AppState,
}

impl ProviderCommand {
    pub fn new(state: AppState) -> Self {
        ProviderCommand { state }
    }

    // Helper function for interactive selection
    async fn select_provider_interactive(&self) -> ReplResult<String> {
        // Get the list of available provider names from AppState
        let providers = self.state.list_providers(); // Assuming list_providers is sync

        if providers.is_empty() {
            // Should ideally not happen if Ollama is always registered
            return Err(ReplError::Provider("No providers registered.".to_string()));
        }

        // Get the current provider name to set the default selection
        let current_provider = self.state.get_provider_name().await;
        let current_index = providers
            .iter()
            .position(|p| p == &current_provider) // <--- FIX IS HERE
            .unwrap_or(0); // Default to the first one if current isn't found (shouldn't happen)

        // Use FuzzySelect for interactive choice
        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select LLM provider (use arrow keys, type to filter)")
            .items(&providers)
            .default(current_index)
            .interact() // This blocks the current thread, but it's fine within the command execution context
            .map_err(|e| ReplError::Command(format!("Provider selection error: {}", e)))?;

        Ok(providers[selection].clone())
    }
}

#[async_trait]
impl Command for ProviderCommand {
    async fn execute(&self, args: &str) -> ReplResult<String> {
        let provider_to_set = if args.trim().is_empty() {
            // No arguments provided, run interactive selection
            self.select_provider_interactive().await?
        } else {
            // Argument provided, use it directly
            args.trim().to_string()
        };

        // Attempt to set the provider in AppState
        self.state.set_provider(&provider_to_set).await?;

        // Return confirmation message
        Ok(format!("Provider set to: {}", provider_to_set))
    }

    fn name(&self) -> &str {
        "provider"
    }

    fn help(&self) -> &str {
        "Select the active LLM provider interactively (/provider) or directly (/provider <name>)"
    }
}