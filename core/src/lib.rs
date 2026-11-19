mod error;
mod history;
mod http;
mod parse;
mod protect;
mod redact;
mod secret_file;
mod settings;
mod template;

pub use error::RewriteError;
pub use history::{HISTORY_LIMIT, HistoryEntry, delete_history, push_history};
pub use http::{
    ChatParams, ChatRequest, ModelInfo, OPENROUTER_BASE, ProviderKind, build_chat_request,
    chat_complete, chat_completions_url, http_client, list_ollama_models, list_openrouter_models,
    ollama_tags_url, openrouter_models_url,
};
pub use parse::{MeasuredPrompt, ParsedPrompt, measure, parse_output};
pub use protect::{
    KeywordCommandOutcome, Protection, corrective_message, format_keyword_text, missing_terms,
    missing_warning, parse_keyword_document, protection_for, run_keyword_line,
    should_retry_protection, strip_inline_braces, system_addon,
};
pub use redact::redact_secrets;
pub use secret_file::{delete_secret_file, read_secret_file, write_secret_file};
pub use settings::{Backend, DEFAULT_HOTKEY, DEFAULT_OLLAMA_URL, DEFAULT_TEMPLATE, Settings};
pub use template::{
    LimitKind, OutputKind, Template, TemplateSummary, build_user_message, bundled_by_id,
    bundled_templates, duration_clock, parse_template, render_system, template_filename,
    valid_template_id, validate_duration, validate_idea,
};
