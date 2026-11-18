use serde::{Deserialize, Serialize};

pub const DEFAULT_HOTKEY: &str = "Ctrl+Shift+Space";
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
pub const DEFAULT_TEMPLATE: &str = "wan22-t2v";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    Openrouter,
    Ollama,
}

impl Default for Backend {
    fn default() -> Self {
        Self::Openrouter
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub backend: Backend,
    pub openrouter_model: String,
    pub ollama_base_url: String,
    pub ollama_model: String,
    pub hotkey: String,
    pub default_template_id: String,
    pub temperature: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            backend: Backend::Openrouter,
            openrouter_model: String::new(),
            ollama_base_url: DEFAULT_OLLAMA_URL.to_string(),
            ollama_model: String::new(),
            hotkey: DEFAULT_HOTKEY.to_string(),
            default_template_id: DEFAULT_TEMPLATE.to_string(),
            temperature: 0.4,
        }
    }
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.temperature = clamp_temperature(self.temperature);
        self.hotkey = self.hotkey.trim().to_string();
        if self.hotkey.is_empty() {
            self.hotkey = DEFAULT_HOTKEY.to_string();
        }
        self.ollama_base_url = self.ollama_base_url.trim().to_string();
        if self.ollama_base_url.is_empty() {
            self.ollama_base_url = DEFAULT_OLLAMA_URL.to_string();
        }
        self.openrouter_model = self.openrouter_model.trim().to_string();
        self.ollama_model = self.ollama_model.trim().to_string();
        self.default_template_id = self.default_template_id.trim().to_string();
        if self.default_template_id.is_empty() {
            self.default_template_id = DEFAULT_TEMPLATE.to_string();
        }
        self
    }
}

pub fn clamp_temperature(value: f32) -> f32 {
    if !value.is_finite() {
        return 0.4;
    }
    (value.clamp(0.0, 2.0) * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_clamp() {
        let settings = Settings::default();
        assert_eq!(settings.backend, Backend::Openrouter);
        assert_eq!(settings.hotkey, "Ctrl+Shift+Space");
        assert_eq!(settings.ollama_base_url, "http://localhost:11434");
        assert_eq!(clamp_temperature(3.2), 2.0);
        assert_eq!(clamp_temperature(-1.0), 0.0);
        assert_eq!(clamp_temperature(f32::NAN), 0.4);
    }
}
