// src/server.rs
use crate::{
    error::ReplError, // Only need ReplError
    state::{AppState, HistoryEntry}, // Only need AppState and HistoryEntry directly
    shell::execute_shell_command,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json as AxumJson, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

// --- Request/Response Structs for API ---
#[derive(Serialize)] struct ApiErrorResponse { error: String, details: Option<String> }
#[derive(Serialize)] struct AppStatusResponse { current_provider: String, current_model: String, markdown_mode: String, theme: String }
#[derive(Serialize)] struct ListResponse<T> { items: Vec<T> }
#[derive(Deserialize)] struct QueryRequest { prompt: String, model: Option<String> }
#[derive(Serialize)] struct QueryResponse { response: String }
#[derive(Deserialize)] struct CommandRequest { command: String }
#[derive(Serialize)] struct CommandResponse { output: String }
#[derive(Deserialize)] struct ShellRequest { command: String }
#[derive(Serialize)] struct ShellResponse { output: String }
#[derive(Serialize)] struct HistoryResponse { history: Vec<HistoryEntry> }

// --- Axum Error Handling ---
enum ApiError { Repl(ReplError), BadRequest(String), NotFound(String) }
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message, details) = match self {
            ApiError::Repl(err) => {
                let status_code = match &err {
                    ReplError::UnknownProvider(_) | ReplError::UnknownCommand(_) => StatusCode::NOT_FOUND,
                    ReplError::Provider(msg) if msg.contains("API key is missing") => StatusCode::UNAUTHORIZED,
                    ReplError::Provider(_) | ReplError::Command(_) => StatusCode::BAD_REQUEST,
                    ReplError::Request(_) | ReplError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
                    ReplError::Json(_) => StatusCode::BAD_REQUEST,
                    ReplError::Readline(_) => StatusCode::INTERNAL_SERVER_ERROR,
                };
                (status_code, err.to_string(), None::<String>) // Provide type hint for None
            }
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg, None::<String>),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg, None::<String>),
        };
        let body = AxumJson(ApiErrorResponse { error: status.canonical_reason().unwrap_or("Error").to_string(), details: Some(error_message) });
        (status, body).into_response()
    }
}
impl From<ReplError> for ApiError { fn from(err: ReplError) -> Self { ApiError::Repl(err) } }

// --- API Handlers ---
async fn get_status(State(state): State<AppState>) -> Result<AxumJson<AppStatusResponse>, ApiError> {
    let status = AppStatusResponse { current_provider: state.get_provider_name().await, current_model: state.get_model().await, markdown_mode: format!("{:?}", state.get_markdown_mode().await), theme: format!("{:?}", state.get_theme().await), }; Ok(AxumJson(status))
}
async fn list_providers(State(state): State<AppState>) -> Result<AxumJson<ListResponse<String>>, ApiError> {
    let providers = state.list_providers(); Ok(AxumJson(ListResponse { items: providers }))
}
async fn list_models( State(state): State<AppState>, Path(provider_name): Path<String>, ) -> Result<AxumJson<ListResponse<String>>, ApiError> {
    let provider = state.get_provider_by_name(&provider_name).ok_or_else(|| ApiError::NotFound(format!("Provider '{}' not found.", provider_name)))?;
    provider.check_readiness().await?; let models = provider.get_models().await?; Ok(AxumJson(ListResponse { items: models }))
}
async fn post_query( State(state): State<AppState>, AxumJson(payload): AxumJson<QueryRequest>, ) -> Result<AxumJson<QueryResponse>, ApiError> {
    let provider_name = state.get_provider_name().await; let provider = state.get_current_provider().await.ok_or_else(|| ApiError::BadRequest(format!("Current provider '{}' is not available or configured.", provider_name)))?;
    let model_to_use = match payload.model { Some(m) => m, None => state.get_model().await, };
    let response_text = provider.query(&model_to_use, &payload.prompt).await?;
    state.add_history_entry(HistoryEntry { entry_type: crate::state::HistoryContentType::LlmResponse { model: model_to_use.clone() }, content: response_text.clone(), }).await;
    Ok(AxumJson(QueryResponse { response: response_text }))
}
async fn post_command( State(state): State<AppState>, AxumJson(payload): AxumJson<CommandRequest>, ) -> Result<AxumJson<CommandResponse>, ApiError> {
    let parts: Vec<&str> = payload.command.trim().splitn(2, ' ').collect(); let (cmd_name, args) = if parts.len() > 1 { (parts[0], parts[1]) } else { (parts[0], "") };
    let command_registry = state.command_registry(); // Get Arc<CommandRegistry>
    let command = command_registry.get_command(cmd_name).ok_or_else(|| ApiError::NotFound(format!("Command '{}' not found.", cmd_name)))?; // Access via Arc
    let output_text = command.execute(args).await?;
    state.add_history_entry(HistoryEntry { entry_type: crate::state::HistoryContentType::CommandResult { command: payload.command.clone() }, content: output_text.clone(), }).await;
    Ok(AxumJson(CommandResponse { output: output_text }))
}
async fn post_shell( State(state): State<AppState>, AxumJson(payload): AxumJson<ShellRequest>, ) -> Result<AxumJson<ShellResponse>, ApiError> {
    let command_line = payload.command.trim();
    if command_line.is_empty() { return Err(ApiError::BadRequest("Shell command cannot be empty.".to_string())); }

    let command_line_owned = command_line.to_string(); // Clone for spawn_blocking
    let command_line_for_history = command_line_owned.clone(); // Clone again for history

    let output_text = tokio::task::spawn_blocking(move || execute_shell_command(&command_line_owned)) // Closure takes ownership of command_line_owned
        .await
        .map_err(|e| ApiError::Repl(ReplError::Command(format!("Shell task join error: {}", e))))??; // Double '?'

    // Use the second clone for the history entry
    state.add_history_entry(HistoryEntry {
        entry_type: crate::state::HistoryContentType::ShellOutput { command: command_line_for_history }, // Use the second clone
        content: output_text.clone(),
    }).await;

    Ok(AxumJson(ShellResponse { output: output_text }))
}
async fn get_history(State(state): State<AppState>) -> Result<AxumJson<HistoryResponse>, ApiError> {
    let history_vec = state.get_history().await; Ok(AxumJson(HistoryResponse { history: history_vec }))
}

// --- Server Setup ---
pub async fn run_server(state: AppState, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).try_init();
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    let app = Router::new()
        .route("/status", get(get_status))
        .route("/providers", get(list_providers))
        .route("/providers/:provider_name/models", get(list_models))
        .route("/query", post(post_query))
        .route("/command", post(post_command))
        .route("/shell", post(post_shell))
        .route("/history", get(get_history))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);
    info!("Starting REST API server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}