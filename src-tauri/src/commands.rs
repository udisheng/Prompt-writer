use crate::files::{self, TemplateCatalog};
use crate::secrets::{self, SecretStatus};
use crate::state::AppState;
use crate::window::{self, present_main};
use rewriter_core::{
    build_chat_request, build_user_message, bundled_by_id, chat_complete, list_ollama_models,
    list_openrouter_models, measure, ollama_tags_url, parse_output, push_history, render_system,
    validate_duration, validate_idea, Backend, ChatParams, HistoryEntry, LimitKind, ModelInfo,
    OutputKind, ProviderKind, Settings, OPENROUTER_BASE,
};
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub backend: Backend,
    pub openrouter_model: String,
    pub ollama_base_url: String,
    pub ollama_model: String,
    pub hotkey: String,
    pub default_template_id: String,
    pub temperature: f32,
    pub key_configured: bool,
    pub secret_store: String,
    pub templates_dir: String,
    pub config_dir: String,
    pub hotkey_active: bool,
    pub hotkey_error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettings {
    pub backend: Backend,
    pub openrouter_model: String,
    pub ollama_base_url: String,
    pub ollama_model: String,
    pub hotkey: String,
    pub default_template_id: String,
    pub temperature: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyInput {
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelQuery {
    pub backend: Backend,
    pub ollama_base_url: Option<String>,
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelList {
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateId {
    pub id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDocument {
    pub id: String,
    pub contents: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateWrite {
    pub id: String,
    pub contents: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteInput {
    pub idea: String,
    pub template_id: String,
    pub duration_secs: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteResponse {
    pub template_id: String,
    pub template_title: String,
    pub output: String,
    pub negative: Option<String>,
    pub char_count: usize,
    pub word_count: usize,
    pub negative_char_count: Option<usize>,
    pub max_chars: Option<usize>,
    pub max_chars_kind: LimitKind,
    pub max_words: Option<usize>,
    pub negative_max_chars: Option<usize>,
    pub output_kind: OutputKind,
    pub over_limit: bool,
    pub warnings: Vec<String>,
    pub history_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryId {
    pub id: String,
}

fn view(app: &AppHandle, state: &AppState, settings: &Settings, secret: &SecretStatus) -> SettingsView {
    let hotkey_error = state.hotkey_error.lock().ok().and_then(|slot| slot.clone());
    SettingsView {
        backend: settings.backend,
        openrouter_model: settings.openrouter_model.clone(),
        ollama_base_url: settings.ollama_base_url.clone(),
        ollama_model: settings.ollama_model.clone(),
        hotkey: settings.hotkey.clone(),
        default_template_id: settings.default_template_id.clone(),
        temperature: settings.temperature,
        key_configured: secret.configured,
        secret_store: secret.store.as_str().to_string(),
        templates_dir: state.templates_dir.display().to_string(),
        config_dir: state.config_dir.display().to_string(),
        hotkey_active: window::hotkey_is_active(app, &settings.hotkey),
        hotkey_error,
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle, state: State<'_, AppState>) -> Result<SettingsView, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings are unavailable.".to_string())?
        .clone();
    let secret = secrets::secret_status(&state.secret_path);
    Ok(view(&app, &state, &settings, &secret))
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, input: SaveSettings) -> Result<SettingsView, String> {
    let next = Settings {
        backend: input.backend,
        openrouter_model: input.openrouter_model,
        ollama_base_url: input.ollama_base_url,
        ollama_model: input.ollama_model,
        hotkey: input.hotkey,
        default_template_id: input.default_template_id,
        temperature: input.temperature,
    }
    .normalized();
    if bundled_by_id(&next.default_template_id).is_err()
        && files::read_template_file(&state.templates_dir, &next.default_template_id).is_err()
    {
        return Err("Choose a default target that has a template file.".into());
    }
    if next.backend == Backend::Ollama {
        ollama_tags_url(&next.ollama_base_url).map_err(|err| err.to_string())?;
    }
    let previous = state
        .settings
        .lock()
        .map_err(|_| "Settings are unavailable.".to_string())?
        .clone();
    if let Err(err) = window::register_hotkey(&app, &next.hotkey) {
        let _ = window::register_hotkey(&app, &previous.hotkey);
        window::remember_hotkey_error(&app, Some(err.clone()));
        return Err(err);
    }
    window::remember_hotkey_error(&app, None);
    files::save_settings(&state.settings_path, &next)?;
    *state
        .settings
        .lock()
        .map_err(|_| "Settings are unavailable.".to_string())? = next.clone();
    let secret = secrets::secret_status(&state.secret_path);
    Ok(view(&app, &state, &next, &secret))
}

#[tauri::command]
pub fn set_openrouter_key(app: AppHandle, state: State<'_, AppState>, input: KeyInput) -> Result<SettingsView, String> {
    secrets::save_key(&state.secret_path, &input.api_key)?;
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings are unavailable.".to_string())?
        .clone();
    let secret = secrets::secret_status(&state.secret_path);
    Ok(view(&app, &state, &settings, &secret))
}

#[tauri::command]
pub fn clear_openrouter_key(app: AppHandle, state: State<'_, AppState>) -> Result<SettingsView, String> {
    secrets::clear_key(&state.secret_path)?;
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings are unavailable.".to_string())?
        .clone();
    let secret = secrets::secret_status(&state.secret_path);
    Ok(view(&app, &state, &settings, &secret))
}

#[tauri::command]
pub async fn list_backend_models(state: State<'_, AppState>, query: ModelQuery) -> Result<ModelList, String> {
    let models = match query.backend {
        Backend::Openrouter => {
            let stored = secrets::load_key(&state.secret_path)?;
            let typed = query.api_key.and_then(|key| {
                let key = key.trim().to_string();
                if key.is_empty() { None } else { Some(key) }
            });
            let key = typed.or(stored);
            list_openrouter_models(&state.http, OPENROUTER_BASE, key.as_deref())
                .await
                .map_err(|err| err.to_string())?
        }
        Backend::Ollama => {
            let base = {
                let settings = state
                    .settings
                    .lock()
                    .map_err(|_| "Settings are unavailable.".to_string())?;
                query
                    .ollama_base_url
                    .filter(|url| !url.trim().is_empty())
                    .unwrap_or_else(|| settings.ollama_base_url.clone())
            };
            list_ollama_models(&state.http, &base)
                .await
                .map_err(|err| err.to_string())?
        }
    };
    Ok(ModelList { models })
}

#[tauri::command]
pub fn list_templates(state: State<'_, AppState>) -> TemplateCatalog {
    files::catalog(&state.templates_dir)
}

#[tauri::command]
pub fn read_template(state: State<'_, AppState>, input: TemplateId) -> Result<TemplateDocument, String> {
    let contents = files::read_template_source(&state.templates_dir, &input.id)?;
    Ok(TemplateDocument {
        id: input.id,
        contents,
    })
}

#[tauri::command]
pub fn save_template(state: State<'_, AppState>, input: TemplateWrite) -> Result<TemplateCatalog, String> {
    files::write_template_source(&state.templates_dir, &input.id, &input.contents)?;
    Ok(files::catalog(&state.templates_dir))
}

#[tauri::command]
pub fn reset_template(state: State<'_, AppState>, input: TemplateId) -> Result<TemplateCatalog, String> {
    let bundled = bundled_by_id(&input.id).map_err(|err| err.to_string())?;
    files::write_template_source(&state.templates_dir, &input.id, bundled)?;
    Ok(files::catalog(&state.templates_dir))
}

#[tauri::command]
pub fn open_templates_dir(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    app.opener()
        .open_path(state.templates_dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|err| format!("Could not open the templates folder: {err}"))
}

#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn show_window(app: AppHandle) {
    present_main(&app);
}

#[tauri::command]
pub async fn rewrite(state: State<'_, AppState>, input: RewriteInput) -> Result<RewriteResponse, String> {
    let idea = validate_idea(&input.idea).map_err(|err| err.to_string())?;
    let path_name = rewriter_core::template_filename(&input.template_id).map_err(|err| err.to_string())?;
    let path = state.templates_dir.join(&path_name);
    if !path.exists() {
        if let Ok(bundled) = bundled_by_id(&input.template_id) {
            fs::write(&path, bundled).map_err(|err| format!("Could not restore the template: {err}"))?;
        }
    }
    let template = files::read_template_file(&state.templates_dir, &input.template_id)?;
    let duration = validate_duration(&template.id, input.duration_secs).map_err(|err| err.to_string())?;
    let system = render_system(&template, duration);
    let user = build_user_message(&template, &idea, duration);
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings are unavailable.".to_string())?
        .clone();
    let (provider, base, model, key) = match settings.backend {
        Backend::Openrouter => {
            let key = secrets::load_key(&state.secret_path)?;
            (
                ProviderKind::Openrouter,
                OPENROUTER_BASE.to_string(),
                settings.openrouter_model.clone(),
                key,
            )
        }
        Backend::Ollama => (
            ProviderKind::Ollama,
            settings.ollama_base_url.clone(),
            settings.ollama_model.clone(),
            None,
        ),
    };
    let request = build_chat_request(ChatParams {
        provider,
        base_url: &base,
        api_key: key.as_deref(),
        model: &model,
        temperature: settings.temperature,
        system: &system,
        user: &user,
    })
    .map_err(|err| err.to_string())?;
    let raw = chat_complete(&state.http, &request).await.map_err(|err| err.to_string())?;
    let measured = measure(&template, parse_output(template.output, &raw));
    let backend_name = match settings.backend {
        Backend::Openrouter => "openrouter",
        Backend::Ollama => "ollama",
    };
    let entry = HistoryEntry::new(
        idea,
        template.id.clone(),
        template.title.clone(),
        duration,
        measured.output.clone(),
        measured.negative.clone(),
        backend_name.to_string(),
        model,
        measured.over_limit,
        measured.warnings.clone(),
    );
    let history_id = entry.id.clone();
    {
        let mut history = state
            .history
            .lock()
            .map_err(|_| "History is unavailable.".to_string())?;
        push_history(&mut history, entry);
        files::save_history(&state.history_path, &history)?;
    }
    Ok(RewriteResponse {
        template_id: template.id,
        template_title: template.title,
        output: measured.output,
        negative: measured.negative,
        char_count: measured.char_count,
        word_count: measured.word_count,
        negative_char_count: measured.negative_char_count,
        max_chars: template.max_chars,
        max_chars_kind: template.max_chars_kind,
        max_words: template.max_words,
        negative_max_chars: template.negative_max_chars,
        output_kind: template.output,
        over_limit: measured.over_limit,
        warnings: measured.warnings,
        history_id,
    })
}

#[tauri::command]
pub fn list_history(state: State<'_, AppState>) -> Result<Vec<HistoryEntry>, String> {
    let history = state
        .history
        .lock()
        .map_err(|_| "History is unavailable.".to_string())?;
    Ok(history.clone())
}

#[tauri::command]
pub fn delete_history_entry(state: State<'_, AppState>, input: HistoryId) -> Result<Vec<HistoryEntry>, String> {
    let mut history = state
        .history
        .lock()
        .map_err(|_| "History is unavailable.".to_string())?;
    rewriter_core::delete_history(&mut history, &input.id);
    files::save_history(&state.history_path, &history)?;
    Ok(history.clone())
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    let mut history = state
        .history
        .lock()
        .map_err(|_| "History is unavailable.".to_string())?;
    history.clear();
    files::save_history(&state.history_path, &history)
}
