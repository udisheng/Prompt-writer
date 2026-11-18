use regex::Regex;
use std::sync::OnceLock;

fn bearer_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)bearer\s+[A-Za-z0-9._\-]{6,}").expect("bearer regex"))
}

fn key_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"sk-[A-Za-z0-9_\-]{6,}").expect("key regex"))
}

/// Remove API keys and bearer tokens from text that might be shown or logged.
pub fn redact_secrets(text: &str, extra: Option<&str>) -> String {
    let mut out = text.to_string();
    if let Some(secret) = extra {
        let secret = secret.trim();
        if secret.len() >= 4 {
            out = out.replace(secret, "[redacted]");
        }
    }
    let out = bearer_re().replace_all(&out, "Bearer [redacted]");
    key_re().replace_all(&out, "sk-[redacted]").into_owned()
}

pub fn truncate_preview(text: &str, max_chars: usize) -> String {
    let redacted = text.trim();
    let mut out = String::new();
    for (i, ch) in redacted.chars().enumerate() {
        if i >= max_chars {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_bearer_sk_and_exact_secret() {
        let raw = "rejected Bearer sk-or-v1-supersecret and also sk-or-v1-supersecret";
        let cleaned = redact_secrets(raw, Some("sk-or-v1-supersecret"));
        assert!(!cleaned.contains("supersecret"));
        assert!(cleaned.contains("[redacted]"));
    }
}
