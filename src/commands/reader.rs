// src/commands/reader.rs
use async_trait::async_trait;
use crate::{
    commands::Command,
    error::ReplResult,
    state::{AppState, HistoryContentType}, // Import history types
    render::get_theme_resources, // For theming the reader output
};
use colored::*; // For coloring headers/separators

pub struct ReaderCommand {
    state: AppState,
}

impl ReaderCommand {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    // Helper to apply theme color
    fn colorize(&self, text: &str, color: (u8, u8, u8)) -> colored::ColoredString {
        text.truecolor(color.0, color.1, color.2)
    }
}

#[async_trait]
impl Command for ReaderCommand {
    async fn execute(&self, _args: &str) -> ReplResult<String> {
        let history = self.state.get_history().await;
        let current_theme = self.state.get_theme().await;
        let (_skin, palette) = get_theme_resources(current_theme); // Use current theme

        // Clear screen or print separator for better view? Optional.
        // print!("\x1B[2J\x1B[1;1H"); // Clears screen - might be too aggressive

        println!("\n{}", self.colorize("--- Session Reader ---", palette.info));
        println!("{}", self.colorize("Scroll through history below.", palette.info));
        println!("{}", self.colorize("Use your terminal's selection feature to copy blocks.", palette.info));
        println!("----------------------\n");


        if history.is_empty() {
            println!("{}", self.colorize("History is empty.", palette.info));
        } else {
            for (index, entry) in history.iter().enumerate() {
                let header_text = match &entry.entry_type {
                    HistoryContentType::LlmResponse { model } => format!("LLM Response ({}) [{}]", model, index + 1),
                    HistoryContentType::CommandResult { command } => format!("Cmd Output (/{} [{}])", command, index + 1),
                    HistoryContentType::ShellOutput { command } => format!("Shell Output (!{} [{}])", command, index + 1),
                    HistoryContentType::UserQuery => format!("User Query [{}]", index + 1),
                    HistoryContentType::Error { source } => format!("Error ({}) [{}]", source, index + 1),
                    HistoryContentType::Info => format!("Info [{}]", index + 1),
                };

                // Print Header with theme color
                println!("{}", self.colorize(&format!("--- {} ---", header_text), palette.prompt_separator)); // Use a distinct color

                // Print the stored content
                // Since we stored the final string (raw or rendered), just print it.
                println!("{}", entry.content.trim()); // Trim potential extra whitespace

                // Print Footer/Separator
                println!("{}\n", self.colorize("--- End ---", palette.prompt_separator));

            }
        }

        println!("{}", self.colorize("--- End of History ---", palette.info));

        // Return a simple confirmation, the main output is printed directly
        Ok("Reader view finished. Scroll up to see history.".to_string())
    }

    fn name(&self) -> &str { "reader" }
    fn help(&self) -> &str { "Display the session output history in a read-only view." }
}