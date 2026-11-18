mod error;
mod history;
mod http;
mod parse;
mod redact;
mod secret_file;
mod settings;
mod template;

pub use error::RewriteError;
pub use history::{delete_history, push_history, HistoryEntry, HISTORY_LIMIT};
pub use http::{
    build_chat_request, chat_complete, chat_completions_url, http_client, list_ollama_models,
    list_openrouter_models, ollama_tags_url, openrouter_models_url, ChatParams, ChatRequest,
    ModelInfo, ProviderKind, OPENROUTER_BASE,
};
pub use parse::{measure, parse_output, MeasuredPrompt, ParsedPrompt};
pub use redact::redact_secrets;
pub use secret_file::{delete_secret_file, read_secret_file, write_secret_file};
pub use settings::{Settings, Backend, DEFAULT_HOTKEY, DEFAULT_OLLAMA_URL, DEFAULT_TEMPLATE};
pub use template::{
    build_user_message, bundled_by_id, bundled_templates, duration_clock, parse_template,
    render_system, template_filename, valid_template_id, validate_duration, validate_idea,
    LimitKind, OutputKind, Template, TemplateSummary,
};
