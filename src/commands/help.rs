// src/commands/help.rs
use async_trait::async_trait;

use crate::{
    commands::Command, // Need Command trait for impl
    error::ReplResult,
    state::{AppState, MarkdownMode}, // Import state elements
};

pub struct HelpCommand {
    state: AppState, // Store state to potentially show status info
}

impl HelpCommand {
    pub fn new(state: AppState) -> Self {
        HelpCommand { state }
    }
}

#[async_trait]
impl Command for HelpCommand {
    async fn execute(&self, _args: &str) -> ReplResult<String> {
        // Get current status for context
        let current_mode = self.state.get_markdown_mode().await;
        let current_theme = self.state.get_theme().await; // Fetch current theme
        let mode_str = match current_mode {
            MarkdownMode::AppendFormatted => "AppendFormatted (Stream raw, append formatted below)",
            MarkdownMode::LiveStreaming => "LiveStreaming (Experimental, may flicker)",
            MarkdownMode::Off => "Off (Raw text output only)",
        };

        // Construct the comprehensive help message
        Ok(format!(r#"
LLM REPL Commands:

  /help                     Show this help message.
  /provider [name]          Select LLM provider (interactive if name omitted).
                            Available: ollama, groq, gemini (check API keys).
  /model [name]             Select model for the current provider (interactive if name omitted).
  /theme [name]             Select theme (interactive if name omitted).
                            Names: default, nord, gruvbox, grayscale.
  /theme_status             Show the current theme ({:?}).
  /md                       Set Markdown Mode: Append Formatted (default).
  /md_streaming             Set Markdown Mode: Live Streaming (Experimental).
  /md_off                   Set Markdown Mode: Off (Raw text).
  /md_status                Show current Markdown mode (Currently: {}).
  /llmconvo                 Start an interactive setup for LLM-to-LLM conversation.
  /reader                   Display the session output history in a read-only view.
  /exit, /quit              Exit the REPL.

Shell Execution:
  !<command> [args]        Execute a shell command (e.g., !ls -l). Output is raw text.

Default Behavior:
  Any other text input is sent as a query to the current LLM provider and model.

Current Theme: {:?}
Current Markdown Mode: {}
"#, current_theme, mode_str, // Placeholders for status
current_theme, mode_str // Actual values for status
        ).trim().to_string())
    }

    fn name(&self) -> &str {
        "help"
    }

    fn help(&self) -> &str {
        "Show this help message."
    }
}