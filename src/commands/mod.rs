// src/commands/mod.rs
use async_trait::async_trait;
use crate::{
    error::ReplResult,
    state::AppState,
};

// Declare the modules for each command
pub mod help;
pub mod llmconvo;
pub mod markdown;
pub mod model;
pub mod provider;
pub mod reader; // Include the reader module
pub mod theme;

/// The core trait that all REPL commands must implement.
#[async_trait]
pub trait Command: Send + Sync {
    /// Executes the command logic.
    async fn execute(&self, args: &str) -> ReplResult<String>;
    /// Returns the name of the command (e.g., "help", "model").
    fn name(&self) -> &str;
    /// Returns a short help string describing the command's purpose.
    fn help(&self) -> &str;
}

/// Holds all registered commands and provides methods to access them.
pub struct CommandRegistry {
    // Store commands as trait objects. Keep private unless necessary.
    commands: Vec<Box<dyn Command>>,
}

impl CommandRegistry {
    /// Creates a new CommandRegistry and registers all built-in commands.
    /// Takes the shared AppState, as commands need it during *their* initialization.
    pub fn new(state: AppState) -> Self {
        let mut registry = CommandRegistry { commands: Vec::new() };

        // --- Register all available commands here ---
        // Pass a clone of AppState to each command constructor that needs it.
        registry.register(Box::new(help::HelpCommand::new(state.clone())));
        registry.register(Box::new(model::ModelCommand::new(state.clone())));
        registry.register(Box::new(provider::ProviderCommand::new(state.clone())));
        registry.register(Box::new(markdown::MdCommand::new(state.clone())));
        registry.register(Box::new(markdown::MdStreamingCommand::new(state.clone())));
        registry.register(Box::new(markdown::MdOffCommand::new(state.clone())));
        registry.register(Box::new(markdown::MdStatusCommand::new(state.clone())));
        registry.register(Box::new(theme::ThemeCommand::new(state.clone())));
        registry.register(Box::new(theme::ThemeStatusCommand::new(state.clone())));
        registry.register(Box::new(llmconvo::LlmConvoCommand::new(state.clone())));
        registry.register(Box::new(reader::ReaderCommand::new(state.clone()))); // Register reader

        registry
    }

    // --- ADDED this empty constructor ---
    /// Creates an empty CommandRegistry (used for temporary state initialization).
    /// This allows AppState::new to create a placeholder before the real registry is built.
    pub fn new_empty() -> Self {
        CommandRegistry { commands: Vec::new() }
    }
    // --- End ADDED ---

    /// Adds a new command to the registry. (Make pub if called externally)
    pub fn register(&mut self, command: Box<dyn Command>) {
        self.commands.push(command);
    }

    /// Finds a command by its name.
    pub fn get_command(&self, name: &str) -> Option<&dyn Command> {
        self.commands.iter().find(|c| c.name() == name).map(|c| &**c)
    }

    /// Returns a list of the names of all registered commands.
    pub fn list_commands(&self) -> Vec<&str> {
        self.commands.iter().map(|c| c.name()).collect()
    }
}