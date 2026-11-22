use rewriter_core::{
    HistoryEntry, RETIRED_MINIMAX_H3_TEMPLATE, Settings, Template, TemplateSummary,
    bundled_templates, parse_template,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidTemplate {
    pub file: String,
    pub error: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateCatalog {
    pub templates: Vec<TemplateSummary>,
    pub invalid: Vec<InvalidTemplate>,
}

#[derive(Serialize, Deserialize, Default)]
struct HistoryFile {
    entries: Vec<HistoryEntry>,
}

pub fn seed_templates(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir)
        .map_err(|err| format!("Could not create the templates folder: {err}"))?;
    let retired = dir.join(format!("{RETIRED_MINIMAX_H3_TEMPLATE}.md"));
    if retired.exists() {
        fs::remove_file(&retired)
            .map_err(|err| format!("Could not remove the retired MiniMax H3 template: {err}"))?;
    }
    for (name, contents) in bundled_templates() {
        let path = dir.join(name);
        if !path.exists() {
            atomic_write(&path, contents.as_bytes())
                .map_err(|err| format!("Could not seed {name}: {err}"))?;
        }
    }
    Ok(())
}

pub fn load_settings(path: &Path) -> Settings {
    let Ok(text) = fs::read_to_string(path) else {
        return Settings::default();
    };
    match serde_json::from_str::<Settings>(&text) {
        Ok(settings) => {
            let retired = settings.default_template_id.trim() == RETIRED_MINIMAX_H3_TEMPLATE;
            let settings = settings.normalized();
            if retired {
                let _ = save_settings(path, &settings);
            }
            settings
        }
        Err(_) => {
            let _ = fs::rename(path, backup_path(path));
            Settings::default()
        }
    }
}

pub fn save_settings(path: &Path, settings: &Settings) -> Result<(), String> {
    let text = serde_json::to_string_pretty(settings).map_err(|err| err.to_string())?;
    atomic_write(path, text.as_bytes()).map_err(|err| format!("Could not save settings: {err}"))
}

pub fn load_history(path: &Path) -> Vec<HistoryEntry> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    if let Ok(file) = serde_json::from_str::<HistoryFile>(&text) {
        return file.entries.into_iter().take(50).collect();
    }
    if let Ok(entries) = serde_json::from_str::<Vec<HistoryEntry>>(&text) {
        return entries.into_iter().take(50).collect();
    }
    let _ = fs::rename(path, backup_path(path));
    Vec::new()
}

pub fn save_history(path: &Path, entries: &[HistoryEntry]) -> Result<(), String> {
    let file = HistoryFile {
        entries: entries.to_vec(),
    };
    let text = serde_json::to_string_pretty(&file).map_err(|err| err.to_string())?;
    atomic_write(path, text.as_bytes()).map_err(|err| format!("Could not save history: {err}"))
}

pub fn catalog(dir: &Path) -> TemplateCatalog {
    let mut templates = Vec::new();
    let mut invalid = Vec::new();
    let Ok(read) = fs::read_dir(dir) else {
        return TemplateCatalog { templates, invalid };
    };
    let mut paths: Vec<PathBuf> = read
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .collect();
    paths.sort();
    for path in paths {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".md") {
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(source) => match parse_template(name, &source) {
                Ok(template) => templates.push(template.summary()),
                Err(err) => invalid.push(InvalidTemplate {
                    file: name.to_string(),
                    error: err.to_string(),
                }),
            },
            Err(err) => invalid.push(InvalidTemplate {
                file: name.to_string(),
                error: err.to_string(),
            }),
        }
    }
    TemplateCatalog { templates, invalid }
}

pub fn read_template_file(dir: &Path, id: &str) -> Result<Template, String> {
    let path = template_path(dir, id)?;
    let source = fs::read_to_string(&path).map_err(|err| format!("Could not read {id}: {err}"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("template.md");
    parse_template(name, &source).map_err(|err| err.to_string())
}

pub fn read_template_source(dir: &Path, id: &str) -> Result<String, String> {
    let path = template_path(dir, id)?;
    fs::read_to_string(&path).map_err(|err| format!("Could not read {id}: {err}"))
}

pub fn write_template_source(dir: &Path, id: &str, contents: &str) -> Result<Template, String> {
    if contents.len() > 100_000 {
        return Err("That template is larger than 100,000 characters.".into());
    }
    let name = rewriter_core::template_filename(id).map_err(|err| err.to_string())?;
    let template = parse_template(&name, contents).map_err(|err| err.to_string())?;
    if template.id != id {
        return Err("The frontmatter id has to match the file you are editing.".into());
    }
    let path = dir.join(name);
    atomic_write(&path, contents.as_bytes())
        .map_err(|err| format!("Could not save the template: {err}"))?;
    Ok(template)
}

fn template_path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    let name = rewriter_core::template_filename(id).map_err(|err| err.to_string())?;
    Ok(dir.join(name))
}

fn backup_path(path: &Path) -> PathBuf {
    let mut backup = path.as_os_str().to_os_string();
    backup.push(".bak");
    PathBuf::from(backup)
}

pub fn load_keyword_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

pub fn save_keyword_text(path: &Path, text: &str) -> Result<(), String> {
    atomic_write(path, text.as_bytes())
        .map_err(|err| format!("Could not save protected keywords: {err}"))
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = fs::remove_file(path);
            fs::rename(&tmp, path).map_err(|_| err)
        }
    }
}
