use crate::error::RewriteError;
use crate::redact::{redact_secrets, truncate_preview};
use crate::settings::clamp_temperature;
use serde::Serialize;
use serde_json::{json, Value};
use std::fmt;
use std::time::Duration;
use url::Url;

pub const OPENROUTER_BASE: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Openrouter,
    Ollama,
}

#[derive(Clone)]
pub struct ChatRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Value,
    provider: ProviderKind,
    model: String,
    endpoint_label: String,
    api_key: Option<String>,
}

impl fmt::Debug for ChatRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let headers: Vec<(String, String)> = self
            .headers
            .iter()
            .map(|(key, value)| {
                if key.eq_ignore_ascii_case("authorization") {
                    (key.clone(), "Bearer [redacted]".to_string())
                } else {
                    (key.clone(), value.clone())
                }
            })
            .collect();
        f.debug_struct("ChatRequest")
            .field("url", &self.url)
            .field("model", &self.model)
            .field("headers", &headers)
            .field("body", &self.body)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub free: bool,
}

pub struct ChatParams<'a> {
    pub provider: ProviderKind,
    pub base_url: &'a str,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
    pub temperature: f32,
    pub system: &'a str,
    pub user: &'a str,
}

pub fn http_client() -> Result<reqwest::Client, RewriteError> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .user_agent("PromptRewriter/0.1")
        .build()
        .map_err(|err| RewriteError::InvalidConfig(format!("Could not build the HTTP client: {err}")))
}

pub fn chat_completions_url(provider: ProviderKind, base: &str) -> Result<String, RewriteError> {
    match provider {
        ProviderKind::Openrouter => join_chat(base),
        ProviderKind::Ollama => ollama_chat_url(base),
    }
}

pub fn ollama_tags_url(base: &str) -> Result<String, RewriteError> {
    let url = parse_http_url(base, "Ollama base URL")?;
    Ok(format!("{}/api/tags", origin(&url)?))
}

pub fn openrouter_models_url(base: &str) -> Result<String, RewriteError> {
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(RewriteError::InvalidConfig("The OpenRouter base URL is empty.".into()));
    }
    if trimmed.ends_with("/models") {
        return Ok(trimmed.to_string());
    }
    if let Some(root) = trimmed.strip_suffix("/chat/completions") {
        return Ok(format!("{}/models", root.trim_end_matches('/')));
    }
    Ok(format!("{trimmed}/models"))
}

pub fn build_chat_request(params: ChatParams<'_>) -> Result<ChatRequest, RewriteError> {
    let model = params.model.trim();
    if model.is_empty() {
        return Err(RewriteError::MissingModel);
    }
    let api_key = params.api_key.map(str::trim).filter(|key| !key.is_empty());
    if params.provider == ProviderKind::Openrouter && api_key.is_none() {
        return Err(RewriteError::MissingApiKey);
    }
    let url = chat_completions_url(params.provider, params.base_url)?;
    let mut headers = Vec::new();
    if params.provider == ProviderKind::Openrouter {
        let key = api_key.unwrap();
        headers.push(("Authorization".into(), format!("Bearer {key}")));
        headers.push(("HTTP-Referer".into(), "https://prompt-rewriter.local".into()));
        headers.push(("X-Title".into(), "Prompt Rewriter".into()));
    }
    let body = json!({
        "model": model,
        "temperature": clamp_temperature(params.temperature),
        "stream": false,
        "messages": [
            {"role": "system", "content": params.system},
            {"role": "user", "content": params.user}
        ]
    });
    Ok(ChatRequest {
        url,
        headers,
        body,
        provider: params.provider,
        model: model.to_string(),
        endpoint_label: params.base_url.trim().to_string(),
        api_key: api_key.map(str::to_string),
    })
}

pub async fn chat_complete(client: &reqwest::Client, request: &ChatRequest) -> Result<String, RewriteError> {
    let mut builder = client.post(&request.url).json(&request.body);
    for (key, value) in &request.headers {
        builder = builder.header(key, value);
    }
    let response = builder.send().await.map_err(|err| {
        transport_error(request.provider, &request.endpoint_label, &err)
    })?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        let preview = truncate_preview(&redact_secrets(&text, request.api_key.as_deref()), 280);
        return Err(map_status(request.provider, status.as_u16(), &preview, &request.model));
    }
    let value: Value = response.json().await.map_err(|_| {
        RewriteError::Provider("The provider returned a response that was not JSON.".into())
    })?;
    extract_completion(&value).ok_or(RewriteError::EmptyCompletion)
}

pub async fn list_openrouter_models(
    client: &reqwest::Client,
    base: &str,
    api_key: Option<&str>,
) -> Result<Vec<ModelInfo>, RewriteError> {
    let url = openrouter_models_url(base)?;
    let key = api_key.map(str::trim).filter(|value| !value.is_empty());
    let first = fetch_models(client, &url, key, ProviderKind::Openrouter).await;
    match first {
        Ok(body) => parse_openrouter_models(&body),
        Err(RewriteError::Unauthorized(_)) if key.is_some() => {
            let body = fetch_models(client, &url, None, ProviderKind::Openrouter).await?;
            parse_openrouter_models(&body)
        }
        Err(err) => Err(err),
    }
}

pub async fn list_ollama_models(client: &reqwest::Client, base: &str) -> Result<Vec<ModelInfo>, RewriteError> {
    let url = ollama_tags_url(base)?;
    let body = fetch_models(client, &url, None, ProviderKind::Ollama).await?;
    parse_ollama_tags(&body)
}

pub fn parse_openrouter_models(body: &str) -> Result<Vec<ModelInfo>, RewriteError> {
    let value: Value = serde_json::from_str(body).map_err(|_| {
        RewriteError::Provider("OpenRouter returned an unexpected model list.".into())
    })?;
    let data = value.get("data").and_then(Value::as_array).ok_or_else(|| {
        RewriteError::Provider("OpenRouter returned an unexpected model list.".into())
    })?;
    let mut models = Vec::new();
    for item in data {
        let Some(id) = item.get("id").and_then(Value::as_str) else {
            continue;
        };
        if id.trim().is_empty() {
            continue;
        }
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(id)
            .trim()
            .to_string();
        let free = item
            .get("pricing")
            .and_then(|pricing| pricing.get("prompt"))
            .map(is_zero)
            .unwrap_or(false);
        models.push(ModelInfo {
            id: id.trim().to_string(),
            name,
            free,
        });
    }
    sort_models(&mut models);
    Ok(models)
}

pub fn parse_ollama_tags(body: &str) -> Result<Vec<ModelInfo>, RewriteError> {
    let value: Value = serde_json::from_str(body).map_err(|_| {
        RewriteError::Provider("Ollama returned an unexpected model list.".into())
    })?;
    let data = value.get("models").and_then(Value::as_array).ok_or_else(|| {
        RewriteError::Provider("Ollama returned an unexpected model list.".into())
    })?;
    let mut models = Vec::new();
    for item in data {
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .or_else(|| item.get("model").and_then(Value::as_str))
            .unwrap_or("")
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }
        models.push(ModelInfo {
            id: name.clone(),
            name,
            free: true,
        });
    }
    sort_models(&mut models);
    Ok(models)
}

fn sort_models(models: &mut [ModelInfo]) {
    models.sort_by(|left, right| {
        right
            .free
            .cmp(&left.free)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn is_zero(value: &Value) -> bool {
    match value {
        Value::String(text) => text.trim().parse::<f64>().ok().is_some_and(|n| n == 0.0),
        Value::Number(number) => number.as_f64().is_some_and(|n| n == 0.0),
        _ => false,
    }
}

async fn fetch_models(
    client: &reqwest::Client,
    url: &str,
    api_key: Option<&str>,
    provider: ProviderKind,
) -> Result<String, RewriteError> {
    let mut builder = client.get(url);
    if let Some(key) = api_key {
        builder = builder.header("Authorization", format!("Bearer {key}"));
    }
    let response = builder.send().await.map_err(|err| transport_error(provider, url, &err))?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        let preview = truncate_preview(&redact_secrets(&text, api_key), 280);
        return Err(map_status(provider, status.as_u16(), &preview, ""));
    }
    Ok(text)
}

fn extract_completion(value: &Value) -> Option<String> {
    let choice = value.get("choices")?.get(0)?;
    if let Some(content) = choice.get("message").and_then(|message| message.get("content")) {
        if let Some(text) = content_to_string(content) {
            if !text.trim().is_empty() {
                return Some(text);
            }
        }
    }
    choice
        .get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn content_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let mut out = String::new();
            for part in parts {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    out.push_str(text);
                } else if let Some(text) = part.as_str() {
                    out.push_str(text);
                }
            }
            if out.trim().is_empty() { None } else { Some(out) }
        }
        _ => None,
    }
}

fn transport_error(provider: ProviderKind, endpoint: &str, err: &reqwest::Error) -> RewriteError {
    if err.is_timeout() {
        return RewriteError::Unreachable(
            "The request timed out. The model may still be loading, or the service did not respond.".into(),
        );
    }
    if err.is_connect() || err.is_request() {
        return match provider {
            ProviderKind::Ollama => RewriteError::Unreachable(format!(
                "Ollama is not running at {endpoint}. Start it with `ollama serve`, or change the base URL in Settings."
            )),
            ProviderKind::Openrouter => RewriteError::Unreachable(
                "Could not reach OpenRouter. Check the network and try again.".into(),
            ),
        };
    }
    RewriteError::Provider("The request failed before a response arrived.".into())
}

fn map_status(provider: ProviderKind, status: u16, preview: &str, model: &str) -> RewriteError {
    let lower = preview.to_ascii_lowercase();
    let model_missing = status == 404
        || (lower.contains("model")
            && (lower.contains("not found")
                || lower.contains("does not exist")
                || lower.contains("unknown")
                || lower.contains("no such")));
    if model_missing {
        return match provider {
            ProviderKind::Ollama => RewriteError::ModelMissing(format!(
                "Ollama does not have model `{model}`. Pull it with `ollama pull {model}` and refresh the list."
            )),
            ProviderKind::Openrouter => RewriteError::ModelMissing(format!(
                "OpenRouter does not have model `{model}`. Pick another model id."
            )),
        };
    }
    match status {
        401 | 403 => match provider {
            ProviderKind::Openrouter => {
                RewriteError::Unauthorized("OpenRouter rejected the API key.".into())
            }
            ProviderKind::Ollama => RewriteError::Unauthorized(format!(
                "Ollama refused the request ({status}). Update Ollama and confirm the model is pulled."
            )),
        },
        429 => RewriteError::Provider(
            "The provider rate-limited the request. Wait a moment and try again.".into(),
        ),
        500..=599 => RewriteError::Provider(format!("The provider returned status {status}. Try again.")),
        _ => {
            let extra = if preview.is_empty() {
                String::new()
            } else {
                format!(" {preview}")
            };
            RewriteError::Provider(format!("The provider returned status {status}.{extra}"))
        }
    }
}

fn join_chat(base: &str) -> Result<String, RewriteError> {
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(RewriteError::InvalidConfig("The API base URL is empty.".into()));
    }
    if trimmed.ends_with("/chat/completions") {
        Ok(trimmed.to_string())
    } else {
        Ok(format!("{trimmed}/chat/completions"))
    }
}

fn ollama_chat_url(base: &str) -> Result<String, RewriteError> {
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        parse_http_url(trimmed, "Ollama base URL")?;
        return Ok(trimmed.to_string());
    }
    let url = parse_http_url(trimmed, "Ollama base URL")?;
    let origin = origin(&url)?;
    let path = url.path().trim_end_matches('/');
    if path == "/v1" || path.ends_with("/v1") {
        Ok(format!("{origin}{path}/chat/completions"))
    } else {
        Ok(format!("{origin}/v1/chat/completions"))
    }
}

fn parse_http_url(base: &str, label: &str) -> Result<Url, RewriteError> {
    let url = Url::parse(base.trim()).map_err(|_| {
        RewriteError::InvalidConfig(format!("{label} is not a valid URL."))
    })?;
    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(RewriteError::InvalidConfig(format!(
                "{label} must start with http:// or https://."
            )))
        }
    }
    if url.host_str().is_none() {
        return Err(RewriteError::InvalidConfig(format!("{label} is missing a host.")));
    }
    Ok(url)
}

fn origin(url: &Url) -> Result<String, RewriteError> {
    let host = url.host_str().ok_or_else(|| {
        RewriteError::InvalidConfig("The URL is missing a host.".into())
    })?;
    match url.port() {
        Some(port) => Ok(format!("{}://{host}:{port}", url.scheme())),
        None => Ok(format!("{}://{host}", url.scheme())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{measure, parse_output};
    use crate::template::{build_user_message, bundled_by_id, parse_template, render_system, OutputKind};
    use serde_json::json;
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn urls_are_normalized() {
        assert_eq!(
            chat_completions_url(ProviderKind::Openrouter, OPENROUTER_BASE).unwrap(),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url(ProviderKind::Ollama, "http://localhost:11434").unwrap(),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url(ProviderKind::Ollama, "http://localhost:11434/v1/").unwrap(),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url(ProviderKind::Ollama, "http://127.0.0.1:11434/v1/chat/completions").unwrap(),
            "http://127.0.0.1:11434/v1/chat/completions"
        );
        assert_eq!(
            ollama_tags_url("http://localhost:11434/v1").unwrap(),
            "http://localhost:11434/api/tags"
        );
        assert!(chat_completions_url(ProviderKind::Ollama, "not a url").is_err());
    }

    #[test]
    fn openrouter_request_keeps_the_key_out_of_the_body_and_debug() {
        let request = build_chat_request(ChatParams {
            provider: ProviderKind::Openrouter,
            base_url: OPENROUTER_BASE,
            api_key: Some("sk-or-v1-supersecret"),
            model: "example/model",
            temperature: 0.4,
            system: "system rules",
            user: "a fox",
        })
        .unwrap();
        assert_eq!(request.body["model"], "example/model");
        assert_eq!(request.body["stream"], false);
        assert_eq!(request.body["messages"][0]["role"], "system");
        assert_eq!(request.body["messages"][1]["content"], "a fox");
        assert!(!request.body.to_string().contains("supersecret"));
        let rendered = format!("{request:?}");
        assert!(!rendered.contains("supersecret"));
        assert!(rendered.contains("[redacted]"));
        assert!(request.headers.iter().any(|(k, v)| k == "Authorization" && v == "Bearer sk-or-v1-supersecret"));
        assert!(build_chat_request(ChatParams {
            provider: ProviderKind::Openrouter,
            base_url: OPENROUTER_BASE,
            api_key: None,
            model: "example/model",
            temperature: 0.2,
            system: "s",
            user: "u",
        })
        .is_err());
    }

    #[test]
    fn ollama_request_has_no_authorization_header() {
        let request = build_chat_request(ChatParams {
            provider: ProviderKind::Ollama,
            base_url: "http://localhost:11434",
            api_key: Some("should-not-be-sent"),
            model: "llama3.2",
            temperature: 0.4,
            system: "s",
            user: "u",
        })
        .unwrap();
        assert!(request.headers.iter().all(|(k, _)| !k.eq_ignore_ascii_case("authorization")));
        assert_eq!(request.url, "http://localhost:11434/v1/chat/completions");
        assert!(build_chat_request(ChatParams {
            provider: ProviderKind::Ollama,
            base_url: "http://localhost:11434",
            api_key: None,
            model: "  ",
            temperature: 0.4,
            system: "s",
            user: "u",
        })
        .is_err());
    }

    #[test]
    fn model_lists_parse_and_sort_free_first() {
        let models = parse_openrouter_models(
            r#"{"data":[{"id":"paid/z","name":"Zed","pricing":{"prompt":"0.1"}},{"id":"free/a","name":"Alpha","pricing":{"prompt":"0"}}]}"#,
        )
        .unwrap();
        assert_eq!(models[0].id, "free/a");
        assert!(models[0].free);
        assert!(!models[1].free);
        let tags = parse_ollama_tags(r#"{"models":[{"name":"llama3.2:latest"}]}"#).unwrap();
        assert_eq!(tags[0].id, "llama3.2:latest");
    }

    #[tokio::test]
    async fn pipeline_posts_the_wan_prompt_and_parses_json() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "{\"positive\":\"Daylight, medium shot. A cat steps along the sill.\",\"negative\":\"blur, subtitles\"}"
                    }
                }]
            })))
            .mount(&server)
            .await;

        let template = parse_template("wan22-t2v.md", bundled_by_id("wan22-t2v").unwrap()).unwrap();
        let idea = "a cat on a windowsill at dawn";
        let system = render_system(&template, None);
        let user = build_user_message(&template, idea, None);
        let request = build_chat_request(ChatParams {
            provider: ProviderKind::Openrouter,
            base_url: &format!("{}/v1", server.uri()),
            api_key: Some("sk-or-v1-supersecret"),
            model: "example/writer",
            temperature: 0.4,
            system: &system,
            user: &user,
        })
        .unwrap();
        let client = reqwest::Client::new();
        let raw = chat_complete(&client, &request).await.unwrap();
        let measured = measure(&template, parse_output(template.output, &raw));
        assert_eq!(measured.output, "Daylight, medium shot. A cat steps along the sill.");
        assert_eq!(measured.negative.as_deref(), Some("blur, subtitles"));
        assert!(!measured.over_limit);
        assert_eq!(template.output, OutputKind::WanPair);

        let seen = server.received_requests().await.unwrap();
        let body: Value = seen[0].body_json().unwrap();
        assert_eq!(body["model"], "example/writer");
        assert_eq!(body["messages"][0]["role"], "system");
        assert!(body["messages"][0]["content"].as_str().unwrap().contains("800"));
        assert!(body["messages"][1]["content"].as_str().unwrap().contains(idea));
        assert!(!body.to_string().contains("supersecret"));
        assert_eq!(
            seen[0].headers.get("authorization").unwrap().to_str().unwrap(),
            "Bearer sk-or-v1-supersecret"
        );
    }

    #[tokio::test]
    async fn bad_key_and_missing_model_and_delimiter_reply() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_string(
                r#"{"error":{"message":"bad key sk-or-v1-supersecret"}}"#,
            ))
            .mount(&server)
            .await;
        let request = build_chat_request(ChatParams {
            provider: ProviderKind::Openrouter,
            base_url: &server.uri(),
            api_key: Some("sk-or-v1-supersecret"),
            model: "missing/model",
            temperature: 0.4,
            system: "s",
            user: "u",
        })
        .unwrap();
        let err = chat_complete(&reqwest::Client::new(), &request).await.unwrap_err();
        let rendered = err.to_string();
        assert!(rendered.contains("API key"));
        assert!(!rendered.contains("supersecret"));

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(400).set_body_string(
                r#"{"error":{"message":"model 'nope' not found"}}"#,
            ))
            .mount(&server)
            .await;
        let request = build_chat_request(ChatParams {
            provider: ProviderKind::Ollama,
            base_url: &server.uri(),
            api_key: None,
            model: "nope",
            temperature: 0.2,
            system: "s",
            user: "u",
        })
        .unwrap();
        let err = chat_complete(&reqwest::Client::new(), &request).await.unwrap_err();
        assert!(matches!(err, RewriteError::ModelMissing(_)));
        assert!(err.to_string().contains("nope"));

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{"message": {"content": "---POSITIVE---\nfixed camera, the cup slides.\n---NEGATIVE---\ncamera shake"}}]
            })))
            .mount(&server)
            .await;
        let template = parse_template("wan22-i2v.md", bundled_by_id("wan22-i2v").unwrap()).unwrap();
        let request = build_chat_request(ChatParams {
            provider: ProviderKind::Ollama,
            base_url: &server.uri(),
            api_key: None,
            model: "qwen2.5",
            temperature: 0.2,
            system: "s",
            user: "u",
        })
        .unwrap();
        let raw = chat_complete(&reqwest::Client::new(), &request).await.unwrap();
        let measured = measure(&template, parse_output(template.output, &raw));
        assert!(measured.output.contains("fixed camera"));
        assert_eq!(measured.negative.as_deref(), Some("camera shake"));
    }

    #[tokio::test]
    async fn ollama_down_is_a_clear_error() {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(400))
            .timeout(Duration::from_millis(800))
            .build()
            .unwrap();
        let request = build_chat_request(ChatParams {
            provider: ProviderKind::Ollama,
            base_url: "http://127.0.0.1:1",
            api_key: None,
            model: "llama3.2",
            temperature: 0.4,
            system: "s",
            user: "u",
        })
        .unwrap();
        let err = chat_complete(&client, &request).await.unwrap_err();
        assert!(err.to_string().contains("Ollama is not running"), "{err}");
    }

    #[tokio::test]
    async fn openrouter_model_list_falls_back_when_the_key_is_rejected() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(wiremock::matchers::header("authorization", "Bearer bad-key"))
            .respond_with(ResponseTemplate::new(401).set_body_string("nope"))
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{"id": "free/gemma", "name": "Gemma", "pricing": {"prompt": "0"}}]
            })))
            .with_priority(5)
            .mount(&server)
            .await;
        let models = list_openrouter_models(
            &reqwest::Client::new(),
            &format!("{}/v1", server.uri()),
            Some("bad-key"),
        )
        .await
        .unwrap();
        assert_eq!(models[0].id, "free/gemma");
        assert!(models[0].free);
    }
}
