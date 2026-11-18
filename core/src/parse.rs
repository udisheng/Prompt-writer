use crate::template::{LimitKind, OutputKind, Template};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPrompt {
    pub output: String,
    pub negative: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasuredPrompt {
    pub output: String,
    pub negative: Option<String>,
    pub char_count: usize,
    pub word_count: usize,
    pub negative_char_count: Option<usize>,
    pub over_limit: bool,
    pub warnings: Vec<String>,
}

pub fn parse_output(kind: OutputKind, raw: &str) -> ParsedPrompt {
    match kind {
        OutputKind::Text => ParsedPrompt {
            output: clean_plain(raw),
            negative: None,
            warnings: Vec::new(),
        },
        OutputKind::WanPair => parse_wan(raw),
    }
}

pub fn measure(template: &Template, parsed: ParsedPrompt) -> MeasuredPrompt {
    let output = parsed.output.trim().to_string();
    let negative = parsed.negative.map(|value| value.trim().to_string()).filter(|value| !value.is_empty());
    let char_count = output.chars().count();
    let word_count = count_words(&output);
    let negative_char_count = negative.as_ref().map(|value| value.chars().count());
    let mut warnings = parsed.warnings;
    let mut over_limit = false;

    if let Some(max) = template.max_chars {
        if char_count > max {
            over_limit = true;
            let noun = match template.max_chars_kind {
                LimitKind::Model => "model limit",
                LimitKind::Suggested => "suggested length",
            };
            warnings.push(format!(
                "The prompt is {char_count} characters, over the {max} character {noun}."
            ));
        }
    }
    if let Some(max) = template.max_words {
        if word_count > max {
            over_limit = true;
            warnings.push(format!(
                "The prompt is {word_count} words, over the {max} word model limit."
            ));
        }
    }
    if let (Some(count), Some(max)) = (negative_char_count, template.negative_max_chars) {
        if count > max {
            over_limit = true;
            warnings.push(format!(
                "The negative prompt is {count} characters, over the {max} character model limit."
            ));
        }
    }

    MeasuredPrompt {
        output,
        negative,
        char_count,
        word_count,
        negative_char_count,
        over_limit,
        warnings,
    }
}

pub fn count_words(text: &str) -> usize {
    text.split_whitespace().filter(|word| !word.is_empty()).count()
}

fn clean_plain(raw: &str) -> String {
    unwrap_fence(raw.trim()).trim().to_string()
}

fn parse_wan(raw: &str) -> ParsedPrompt {
    let trimmed = raw.trim();
    let unfenced = unwrap_fence(trimmed).trim();
    if let Some(parsed) = wan_from_json(unfenced).or_else(|| wan_from_json(trimmed)) {
        return parsed;
    }
    if let Some((positive, negative)) = split_markers(unfenced) {
        return ParsedPrompt {
            output: positive,
            negative: Some(negative),
            warnings: Vec::new(),
        };
    }
    ParsedPrompt {
        output: unfenced.to_string(),
        negative: None,
        warnings: vec![
            "Could not split a positive and negative prompt. The raw reply is in the positive box."
                .into(),
        ],
    }
}

fn wan_from_json(text: &str) -> Option<ParsedPrompt> {
    let candidate = extract_object(text)?;
    let value: Value = serde_json::from_str(&candidate).ok()?;
    let obj = value.as_object()?;
    let positive = first_string(obj, &["positive", "positive_prompt", "prompt"])?;
    if positive.trim().is_empty() {
        return None;
    }
    let negative = first_string(obj, &["negative", "negative_prompt"]).unwrap_or_default();
    Some(ParsedPrompt {
        output: positive.trim().to_string(),
        negative: if negative.trim().is_empty() {
            None
        } else {
            Some(negative.trim().to_string())
        },
        warnings: Vec::new(),
    })
}

fn first_string(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(Value::String(value)) = obj.get(*key) {
            return Some(value.clone());
        }
    }
    None
}

fn extract_object(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut start = None;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (index, &byte) in bytes.iter().enumerate() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if byte == b'\\' {
                escape = true;
                continue;
            }
            if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth += 1;
            }
            b'}' => {
                if depth == 0 {
                    continue;
                }
                depth -= 1;
                if depth == 0 {
                    let start = start?;
                    return Some(text[start..=index].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn unwrap_fence(text: &str) -> &str {
    let trimmed = text.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    let rest = rest
        .strip_prefix("json")
        .or_else(|| rest.strip_prefix("JSON"))
        .unwrap_or(rest);
    let rest = rest.trim_start_matches(['\r', '\n', ' ']);
    match rest.rfind("```") {
        Some(end) if end > 0 => rest[..end].trim(),
        _ => trimmed,
    }
}

fn split_markers(text: &str) -> Option<(String, String)> {
    let pairs = [
        ("---POSITIVE---", "---NEGATIVE---"),
        ("POSITIVE:", "NEGATIVE:"),
    ];
    let lower = text.to_ascii_lowercase();
    for (positive_marker, negative_marker) in pairs {
        let positive_marker = positive_marker.to_ascii_lowercase();
        let negative_marker = negative_marker.to_ascii_lowercase();
        let Some(pos_at) = line_marker(&lower, &positive_marker) else {
            continue;
        };
        let after_pos = pos_at + positive_marker.len();
        let Some(neg_rel) = line_marker(&lower[after_pos..], &negative_marker) else {
            continue;
        };
        let neg_at = after_pos + neg_rel;
        let positive = text[after_pos..neg_at].trim().to_string();
        let negative = text[neg_at + negative_marker.len()..].trim().to_string();
        if positive.is_empty() {
            continue;
        }
        return Some((positive, negative));
    }
    None
}

fn line_marker(haystack: &str, marker: &str) -> Option<usize> {
    let mut offset = 0;
    for line in haystack.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let leading = line.len() - trimmed.len();
        if trimmed.starts_with(marker) {
            return Some(offset + leading);
        }
        offset += line.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::{parse_template, bundled_by_id};

    fn wan() -> Template {
        parse_template("wan22-t2v.md", bundled_by_id("wan22-t2v").unwrap()).unwrap()
    }

    #[test]
    fn parses_raw_json_fenced_json_and_prose_wrapped_json() {
        let raw = r#"{"positive":"A cat walks.","negative":"blur, watermark"}"#;
        let parsed = parse_output(OutputKind::WanPair, raw);
        assert_eq!(parsed.output, "A cat walks.");
        assert_eq!(parsed.negative.as_deref(), Some("blur, watermark"));

        let fenced = "```json\n{\"positive_prompt\":\"Motion only.\",\"negative_prompt\":\"shake\"}\n```";
        let parsed = parse_output(OutputKind::WanPair, fenced);
        assert_eq!(parsed.output, "Motion only.");
        assert_eq!(parsed.negative.as_deref(), Some("shake"));

        let wrapped = "Here is the prompt:\n{\"prompt\":\"Edge lighting, a fox turns.\",\"negative\":\"subtitles\"}\nThanks!";
        let parsed = parse_output(OutputKind::WanPair, wrapped);
        assert_eq!(parsed.output, "Edge lighting, a fox turns.");
        assert_eq!(parsed.negative.as_deref(), Some("subtitles"));
    }

    #[test]
    fn parses_delimiters_and_falls_back() {
        let text = "---POSITIVE---\nThe rider leans.\n---NEGATIVE---\nblur, extra fingers";
        let parsed = parse_output(OutputKind::WanPair, text);
        assert_eq!(parsed.output, "The rider leans.");
        assert_eq!(parsed.negative.as_deref(), Some("blur, extra fingers"));

        let labeled = "POSITIVE:\nfixed camera, the curtain lifts.\nNEGATIVE:\ncamera shake";
        let parsed = parse_output(OutputKind::WanPair, labeled);
        assert!(parsed.output.contains("fixed camera"));
        assert_eq!(parsed.negative.as_deref(), Some("camera shake"));

        let fallback = parse_output(OutputKind::WanPair, "just a sentence with no split");
        assert!(fallback.warnings.iter().any(|w| w.contains("raw reply")));
        assert!(fallback.negative.is_none());
    }

    #[test]
    fn plain_text_unwraps_a_fence_and_counts_limits() {
        let template = parse_template("seedream-generate.md", bundled_by_id("seedream-generate").unwrap()).unwrap();
        let parsed = parse_output(OutputKind::Text, "```\nA child runs through tall grass.\n```");
        assert_eq!(parsed.output, "A child runs through tall grass.");
        let measured = measure(&template, parsed);
        assert!(!measured.over_limit);

        let template = wan();
        let long = "word ".repeat(200);
        let parsed = ParsedPrompt {
            output: long,
            negative: Some("x".repeat(501)),
            warnings: vec![],
        };
        let measured = measure(&template, parsed);
        assert!(measured.over_limit);
        assert!(measured.warnings.len() >= 2);
    }

    #[test]
    fn json_string_braces_do_not_end_the_object_early() {
        let raw = r#"{"positive":"a sign reading \"{open}\".","negative":"blur"}"#;
        let parsed = parse_output(OutputKind::WanPair, raw);
        assert!(parsed.output.contains("{open}"));
        assert_eq!(parsed.negative.as_deref(), Some("blur"));
    }

}
