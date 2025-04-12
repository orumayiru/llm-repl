// src/commands/theme.rs
use async_trait::async_trait;
use dialoguer::{theme::ColorfulTheme, Select};
// Ensure strum imports are present if you haven't added them to Cargo.toml yet
use strum::IntoEnumIterator;
use strum_macros::EnumIter; 

use crate::{
    commands::Command,
    error::{ReplError, ReplResult},
    state::{AppState, RenderTheme}, // Import state RenderTheme
};

// --- Define a local enum for Dialoguer interaction ---
// Derives allow iterating over variants and displaying them
#[derive(Debug, Clone, Copy, PartialEq, EnumIter)]
pub enum SelectableTheme {
    Default,
    Nord,
    Gruvbox,
    Grayscale,
}

// How the themes will be displayed in the selection list
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

// Map the selection back to the RenderTheme enum used in AppState
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

// Map the AppState::RenderTheme to the index for the SelectableTheme list
fn theme_to_index(state_theme: RenderTheme) -> usize {
     match state_theme {
        RenderTheme::Default => 0,
        RenderTheme::Nord => 1,
        RenderTheme::Gruvbox => 2,
        RenderTheme::Grayscale => 3,
        // Add future themes here
    }
}


// --- Unified /theme Command Implementation ---
#[derive(Clone)]
pub struct ThemeCommand {
    state: AppState,
}

impl ThemeCommand {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    // Helper function for the interactive selection dialog
    async fn select_theme_interactive(&self) -> ReplResult<RenderTheme> {
        // Create a list of selectable theme variants
        let themes: Vec<SelectableTheme> = SelectableTheme::iter().collect();

        // Get the current theme from AppState to set the default selection
        let current_state_theme = self.state.get_theme().await;
        let current_index = theme_to_index(current_state_theme); // Use helper

        // Show the selection dialog
        let selection_index = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select Markdown rendering theme")
            .items(&themes) // Display variants using their Display impl
            .default(current_index)
            .interact() // Blocks for user input
            .map_err(|e| ReplError::Command(format!("Theme selection error: {}", e)))?;

        // Convert the selected index back to the corresponding AppState::RenderTheme
        Ok(RenderTheme::from(themes[selection_index]))
    }
}

#[async_trait]
impl Command for ThemeCommand {
    async fn execute(&self, args: &str) -> ReplResult<String> {
        let theme_to_set = if args.trim().is_empty() {
            // No arguments: Run interactive selection
            self.select_theme_interactive().await?
        } else {
            // Argument provided: Parse it
            let arg_lower = args.trim().to_lowercase();
            match arg_lower.as_str() {
                "default" => RenderTheme::Default,
                "nord" => RenderTheme::Nord,
                "gruvbox" => RenderTheme::Gruvbox,
                "grayscale" => RenderTheme::Grayscale,
                // Add aliases if desired (e.g., "grey" for "grayscale")
                _ => {
                    // Argument didn't match known themes
                    return Err(ReplError::Command(format!(
                        "Unknown theme '{}'. Available: default, nord, gruvbox, grayscale", args
                    )));
                }
            }
        };

        // Set the chosen theme in AppState
        self.state.set_theme(theme_to_set).await;

        // Return confirmation message
        Ok(format!("Markdown theme set to: {:?}", theme_to_set)) // Use Debug formatting
    }

    fn name(&self) -> &str {
        "theme" // Command is invoked with /theme
    }

    fn help(&self) -> &str {
        "Select Markdown theme interactively (/theme) or by name (/theme <default|nord|gruvbox|grayscale>)"
    }
}


// --- Status Command (Keep as is) ---
#[derive(Clone)]
pub struct ThemeStatusCommand { state: AppState }
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