// src/commands/model.rs
use async_trait::async_trait;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};

use crate::{
    commands::Command,
    error::{ReplError, ReplResult},
    state::AppState,
};

#[derive(Clone)]
pub struct ModelCommand {
    state: AppState,
}

impl ModelCommand {
    pub fn new(state: AppState) -> Self {
        ModelCommand { state }
    }

    async fn select_model_interactive(&self) -> ReplResult<String> {
        let models = self.state.list_models().await?;
        
        if models.is_empty() {
            return Err(ReplError::Provider("No models available".to_string()));
        }

        let current_model = self.state.get_model().await;
        let current_index = models.iter()
            .position(|m| m == &current_model)
            .unwrap_or(0);

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select model (use arrow keys, type to filter)")
            .items(&models)
            .default(current_index)
            .interact()
            .map_err(|e| ReplError::Command(format!("Selection error: {}", e)))?;

        Ok(models[selection].clone())
    }
}

#[async_trait]
impl Command for ModelCommand {
    async fn execute(&self, args: &str) -> ReplResult<String> {
        let trimmed_args = args.trim();

        if trimmed_args.is_empty() {
            // --- Interactive Mode ---
            // This mode inherently selects from the available list.
            let selected_model = self.select_model_interactive().await?;
            self.state.set_model(&selected_model).await?; // Set the validated model
            Ok(format!("Model set to: {}", selected_model))

        } else {
            // --- Direct Setting Mode (with Validation) ---
            let proposed_model = trimmed_args;
            let provider_name = self.state.get_provider_name().await; // Get for context

            // Fetch the available models for the *current* provider
            let available_models = match self.state.list_models().await {
                 Ok(models) => models,
                 Err(e) => {
                     // Handle error during model list fetching
                     return Err(ReplError::Command(format!(
                         "Could not fetch models for provider '{}' to validate: {}. Cannot set model.",
                         provider_name, e
                     )));
                 }
            };

            // Check if the proposed model exists in the fetched list
            if available_models.iter().any(|m| m == proposed_model) {
                // Model is valid, set it in the state
                self.state.set_model(proposed_model).await?;
                Ok(format!("Model set to: {}", proposed_model))
            } else {
                // Model is invalid, return an error
                // Suggest using interactive mode
                Err(ReplError::Command(format!(
                    "Model '{}' not found for current provider '{}'. Use '/model' without arguments to see available models.",
                    proposed_model, provider_name
                )))
            }
        }
    } // --- End execute method ---

    fn name(&self) -> &str {
        "model"
    }

    fn help(&self) -> &str {
        "Select a model (interactively with /model or directly with /model <name>)"
    }
}