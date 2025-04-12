// src/commands/llmconvo.rs
use async_trait::async_trait;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input, Editor, Select};
use std::io::{self, Write};
// Removed unused Arc: use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use colored::Colorize;
use futures::StreamExt;
// Removed unused IntoEnumIterator: use strum::IntoEnumIterator;
use strum_macros::EnumIter; // Still need EnumIter for derive

use crate::{
    commands::Command,
    error::{ReplError, ReplResult},
    providers::LlmProvider,
    state::{AppState, MarkdownMode, RenderTheme},
    render::{get_theme_resources}, // Removed unused ThemePalette import here
    signal::{is_stop_requested, reset_stop_flag},
};

// Structure to hold LLM instance details
#[derive(Clone)]
struct LlmInstance {
    provider: Box<dyn LlmProvider>,
    model: String,
    persona: String,
}

// Structure for conversation messages
#[derive(Clone, Debug)]
struct ConvoMessage {
    role: String, // "system", "user", "LLM_1", "LLM_2"
    content: String,
}

// --- Define a local enum for Dialoguer interaction ---
#[derive(Debug, Clone, Copy, PartialEq, EnumIter)] // Need EnumIter derive
pub enum SelectableTheme {
    Default,
    Nord,
    Gruvbox,
    Grayscale,
}

impl std::fmt::Display for SelectableTheme {
     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectableTheme::Default => write!(f, "Default (Termimad Default)"),
            SelectableTheme::Nord => write!(f, "Nord (Cool, subdued blues)"),
            SelectableTheme::Gruvbox => write!(f, "Gruvbox (Warm retro - WIP)"),
            SelectableTheme::Grayscale => write!(f, "Grayscale (Minimal - WIP)"),
        }
    }
}

impl From<SelectableTheme> for RenderTheme {
    fn from(selectable: SelectableTheme) -> Self {
        match selectable {
            SelectableTheme::Default => RenderTheme::Default,
            SelectableTheme::Nord => RenderTheme::Nord,
            SelectableTheme::Gruvbox => RenderTheme::Gruvbox,
            SelectableTheme::Grayscale => RenderTheme::Grayscale,
        }
    }
}

fn theme_to_index(state_theme: RenderTheme) -> usize {
     match state_theme {
        RenderTheme::Default => 0,
        RenderTheme::Nord => 1,
        RenderTheme::Gruvbox => 2,
        RenderTheme::Grayscale => 3,
    }
}
// --- End Theme Selection Helpers ---


#[derive(Clone)]
pub struct LlmConvoCommand {
    state: AppState,
}

impl LlmConvoCommand {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    // Helper to select provider and model
    async fn select_llm_instance(&self, instance_name: &str) -> ReplResult<LlmInstance> {
        println!("--- Configure {} LLM ---", instance_name);

        let providers = self.state.list_providers();
        if providers.is_empty() { return Err(ReplError::Command("No LLM providers available.".to_string())); }
        let provider_selection_index = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Select provider for {} LLM", instance_name))
            .items(&providers).default(0).interact().map_err(ReplError::from)?;
        let provider_name = &providers[provider_selection_index];

        let provider = self.state.get_provider_by_name(provider_name)
            .ok_or_else(|| ReplError::UnknownProvider(provider_name.to_string()))?;
        provider.check_readiness().await.map_err(|e|
            ReplError::Provider(format!("Provider '{}' is not ready: {}", provider_name, e))
        )?;

        let models = provider.get_models().await.map_err(|e|
            ReplError::Command(format!("Could not list models for provider '{}': {}", provider_name, e))
        )?;
        if models.is_empty() { return Err(ReplError::Command(format!("No models available for provider '{}'.", provider_name))); }
        let model_selection_index = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Select model for {} LLM (Provider: {})", instance_name, provider_name))
            .items(&models).default(0).interact().map_err(ReplError::from)?;
        let model_name = models[model_selection_index].clone();

        println!("Define persona/instructions for {} LLM.", instance_name);
        println!("(Describe its role, personality, goals. End with Enter then Ctrl+D/Ctrl+Z)");
        let persona = Editor::new()
                   .edit("Enter persona description...")
                   .map_err(ReplError::from)?
                   .unwrap_or_default();

        Ok(LlmInstance { provider, model: model_name, persona })
    }

    // Helper to get conversation parameters
    fn get_conversation_parameters(&self) -> ReplResult<(u32, String)> {
         println!("--- Configure Conversation ---");
         let turns: u32 = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter max number of conversation turns (e.g., 10)")
            .validate_with(|input: &String| -> Result<(), &str> {
                 // Corrected validation logic
                input.trim().parse::<u32>()
                    .map_err(|_| "Not a valid number") // Use map_err for parse fail
                    .and_then(|n| if n > 0 { Ok(()) } else { Err("Must be > 0") }) // Chain positive check
                    .map_err(|e| if e == "Must be > 0" { e } else { "Please enter a valid positive number" }) // Final error message consolidation
            })
            .interact_text().map_err(ReplError::from)?
            .trim().parse::<u32>().expect("Validated input failed parse");

         println!("Enter the initial topic or starting prompt for the conversation.");
         println!("(End with Enter then Ctrl+D/Ctrl+Z)");
         let topic = Editor::new()
                   .edit("Enter topic...")
                   .map_err(ReplError::from)?
                   .unwrap_or_default();

        Ok((turns, topic))
    }

    // --- The Core Conversation Loop ---
    async fn run_conversation_loop(
        &self,
        mut llm1: LlmInstance,
        mut llm2: LlmInstance,
        initial_topic: String,
        max_turns: u32,
        _markdown_mode: MarkdownMode, // Marked unused
        theme: RenderTheme,
    ) -> ReplResult<()> {
        let (_skin, palette) = get_theme_resources(theme);

        println!("\n--- Starting Conversation ---");
        println!("LLM 1 ({} - {}): {}", llm1.provider.get_name(), llm1.model, llm1.persona.lines().next().unwrap_or("..."));
        println!("LLM 2 ({} - {}): {}", llm2.provider.get_name(), llm2.model, llm2.persona.lines().next().unwrap_or("..."));
        println!("Topic: {}", initial_topic.lines().next().unwrap_or("..."));
        println!("Max Turns: {}", max_turns);
        println!("{}", "Press Ctrl+C between turns to stop.".truecolor(palette.info.0, palette.info.1, palette.info.2));
        println!("-----------------------------\n");

        reset_stop_flag();

        let mut history: Vec<ConvoMessage> = Vec::new();
        history.push(ConvoMessage { role: "system".to_string(), content: llm1.persona.clone() });
        history.push(ConvoMessage { role: "user".to_string(), content: initial_topic });

        let mut current_speaker_idx = 0;

        for turn in 0..max_turns {
            if is_stop_requested() {
                println!("\n{}", "[ Conversation Interrupted ]".truecolor(palette.error.0, palette.error.1, palette.error.2));
                reset_stop_flag();
                return Ok(());
            }

            let (current_llm, speaker_role_str) = if current_speaker_idx == 0 {
                (&mut llm1, "LLM_1")
            } else {
                (&mut llm2, "LLM_2")
            };

            println!( "\n{}", format!( "-- Turn {} | {} ({}:{}) thinking... --", turn + 1, speaker_role_str, current_llm.provider.get_name(), current_llm.model)
                .truecolor(palette.info.0, palette.info.1, palette.info.2) );

            // !! Simplified Prompt Preparation !!
            let prompt_text = history.iter()
                .map(|msg| format!("{}: {}", msg.role, msg.content))
                .collect::<Vec<_>>().join("\n\n");

            let response_result = match current_llm.provider.query_stream(&current_llm.model, &prompt_text).await {
                Ok(Some(stream)) => {
                    print!("{}: ", speaker_role_str.truecolor(palette.success.0, palette.success.1, palette.success.2));
                    let mut full_response = String::new();
                    let mut stream_pin = stream;
                    while let Some(chunk_res) = stream_pin.next().await {
                        match chunk_res {
                            Ok(chunk) => {
                                print!("{}", chunk);
                                io::stdout().flush().map_err(ReplError::Io)?;
                                full_response.push_str(&chunk);
                            }
                            Err(e) => {
                                eprintln!("\n{}", format!("Stream error during {}'s turn: {}", speaker_role_str, e).truecolor(palette.error.0, palette.error.1, palette.error.2));
                                full_response.push_str(" [ Stream error ]");
                                break;
                            }
                        }
                    }
                    println!();
                    Ok(full_response)
                },
                Ok(None) | Err(_) => {
                    print!("{}: ", speaker_role_str.truecolor(palette.success.0, palette.success.1, palette.success.2));
                    match current_llm.provider.query(&current_llm.model, &prompt_text).await {
                         Ok(response) => {
                             println!("{}", response.trim());
                             Ok(response)
                         },
                         Err(e) => Err(e),
                    }
                }
            };

            match response_result {
                Ok(response_content) => {
                    history.push(ConvoMessage {
                        role: speaker_role_str.to_string(),
                        content: response_content.trim().to_string(),
                    });
                }
                Err(e) => {
                     eprintln!( "\n{}", format!("Error getting response for {}'s turn: {}", speaker_role_str, e)
                         .truecolor(palette.error.0, palette.error.1, palette.error.2) );
                     println!("{}", "[ Conversation ended due to error ]".truecolor(palette.error.0, palette.error.1, palette.error.2));
                     reset_stop_flag();
                     return Err(e);
                }
            }

            current_speaker_idx = 1 - current_speaker_idx;
            sleep(Duration::from_millis(200)).await;
        }

        // Use palette for final success message
        println!("\n{}", "[ Conversation Finished - Max Turns Reached ]".truecolor(palette.success.0, palette.success.1, palette.success.2));
        reset_stop_flag();
        Ok(())
    }
}


#[async_trait]
impl Command for LlmConvoCommand {
    async fn execute(&self, _args: &str) -> ReplResult<String> {
        println!("{}", "Starting LLM Conversation setup...".yellow());

        let llm1 = self.select_llm_instance("first").await?;
        let llm2 = self.select_llm_instance("second").await?;
        let (max_turns, topic) = self.get_conversation_parameters()?;

        let markdown_mode = self.state.get_markdown_mode().await;
        let theme = self.state.get_theme().await;
        // Removed unused palette fetch here: let (_skin, palette) = get_theme_resources(theme);

        if let Err(e) = self.run_conversation_loop(
                llm1, llm2, topic, max_turns, markdown_mode, theme
            ).await {
             return Err(ReplError::Command(format!("Conversation ended with error: {}", e)));
        }

        // Success message now printed inside run_conversation_loop
        Ok("Conversation completed.".to_string()) // Return simple confirmation string
    }

    fn name(&self) -> &str { "llmconvo" }
    fn help(&self) -> &str { "Start a conversation between two configured LLMs." }
}

// Keep ThemeStatusCommand
#[derive(Clone)] pub struct ThemeStatusCommand { state: AppState }
impl ThemeStatusCommand { pub fn new(state: AppState) -> Self { Self { state } } }
#[async_trait]
impl Command for ThemeStatusCommand {
    async fn execute(&self, _args: &str) -> ReplResult<String> {
        let theme = self.state.get_theme().await;
        Ok(format!("Current Markdown theme: {:?}", theme))
    }
    fn name(&self) -> &str { "theme_status" }
    fn help(&self) -> &str { "Show the current Markdown rendering theme." }
 }