use rewriter_core::{HistoryEntry, Settings};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub http: reqwest::Client,
    pub config_dir: PathBuf,
    pub templates_dir: PathBuf,
    pub settings_path: PathBuf,
    pub history_path: PathBuf,
    pub secret_path: PathBuf,
    pub keywords_path: PathBuf,
    pub settings: Mutex<Settings>,
    pub history: Mutex<Vec<HistoryEntry>>,
    pub hotkey_error: Mutex<Option<String>>,
}
