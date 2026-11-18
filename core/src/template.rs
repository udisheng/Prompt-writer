use crate::error::RewriteError;
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

static BUNDLED: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    Text,
    WanPair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LimitKind {
    Model,
    Suggested,
}

impl Default for LimitKind {
    fn default() -> Self {
        Self::Model
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub id: String,
    pub family: String,
    pub variant: String,
    pub title: String,
    pub max_chars: Option<usize>,
    pub max_chars_kind: LimitKind,
    pub max_words: Option<usize>,
    pub negative_max_chars: Option<usize>,
    pub output: OutputKind,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSummary {
    pub id: String,
    pub family: String,
    pub variant: String,
    pub title: String,
    pub max_chars: Option<usize>,
    pub max_chars_kind: LimitKind,
    pub max_words: Option<usize>,
    pub negative_max_chars: Option<usize>,
    pub output: OutputKind,
}

impl Template {
    pub fn summary(&self) -> TemplateSummary {
        TemplateSummary {
            id: self.id.clone(),
            family: self.family.clone(),
            variant: self.variant.clone(),
            title: self.title.clone(),
            max_chars: self.max_chars,
            max_chars_kind: self.max_chars_kind,
            max_words: self.max_words,
            negative_max_chars: self.negative_max_chars,
            output: self.output,
        }
    }
}

pub fn valid_template_id(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphanumeric()) {
        return false;
    }
    if id.len() > 64 || id.contains("..") || id.contains('/') || id.contains('\\') {
        return false;
    }
    id.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

pub fn template_filename(id: &str) -> Result<String, RewriteError> {
    if !valid_template_id(id) {
        return Err(RewriteError::Template(format!("Unknown template id `{id}`.")));
    }
    Ok(format!("{id}.md"))
}

pub fn bundled_templates() -> Vec<(&'static str, &'static str)> {
    let mut files: Vec<_> = BUNDLED
        .files()
        .filter_map(|file| {
            let name = file.path().file_name()?.to_str()?;
            if !name.ends_with(".md") {
                return None;
            }
            Some((name, file.contents_utf8()?))
        })
        .collect();
    files.sort_by_key(|(name, _)| *name);
    files
}

pub fn bundled_by_id(id: &str) -> Result<&'static str, RewriteError> {
    let filename = template_filename(id)?;
    bundled_templates()
        .into_iter()
        .find(|(name, _)| *name == filename)
        .map(|(_, body)| body)
        .ok_or_else(|| RewriteError::Template(format!("No bundled template named `{id}`.")))
}

pub fn parse_template(filename: &str, source: &str) -> Result<Template, RewriteError> {
    let source = source.trim_start_matches('\u{feff}');
    let (meta, body) = split_frontmatter(filename, source)?;
    let id = required(&meta, "id", filename)?;
    let stem = filename
        .trim_end_matches(".md")
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename);
    if id != stem {
        return Err(RewriteError::Template(format!(
            "{filename}: frontmatter id `{id}` does not match the file name."
        )));
    }
    if !valid_template_id(&id) {
        return Err(RewriteError::Template(format!(
            "{filename}: template id `{id}` is not a safe file name."
        )));
    }
    let output = match required(&meta, "output", filename)?.as_str() {
        "text" => OutputKind::Text,
        "wan_pair" => OutputKind::WanPair,
        other => {
            return Err(RewriteError::Template(format!(
                "{filename}: output `{other}` must be text or wan_pair."
            )))
        }
    };
    let max_chars_kind = match meta.get("max_chars_kind").map(String::as_str) {
        None => LimitKind::Model,
        Some("model") => LimitKind::Model,
        Some("suggested") => LimitKind::Suggested,
        Some(other) => {
            return Err(RewriteError::Template(format!(
                "{filename}: max_chars_kind `{other}` must be model or suggested."
            )))
        }
    };
    Ok(Template {
        id,
        family: required(&meta, "family", filename)?,
        variant: required(&meta, "variant", filename)?,
        title: required(&meta, "title", filename)?,
        max_chars: optional_usize(&meta, "max_chars", filename)?,
        max_chars_kind,
        max_words: optional_usize(&meta, "max_words", filename)?,
        negative_max_chars: optional_usize(&meta, "negative_max_chars", filename)?,
        output,
        body: body.trim().to_string(),
    })
}

pub fn duration_clock(seconds: u32) -> String {
    format!("00:{:02}.000", seconds.min(59))
}

pub fn duration_bounds(template_id: &str) -> Option<(&'static str, u32, u32)> {
    match template_id {
        "minimax-h3" => Some(("MiniMax H3", 4, 15)),
        "seedance-2.5" => Some(("Seedance 2.5", 4, 30)),
        id if id.starts_with("seedance-") => Some(("Seedance", 4, 15)),
        _ => None,
    }
}

pub fn validate_duration(template_id: &str, duration: Option<u32>) -> Result<Option<u32>, RewriteError> {
    let Some((label, min, max)) = duration_bounds(template_id) else {
        return Ok(None);
    };
    match duration {
        Some(secs) if (min..=max).contains(&secs) => Ok(Some(secs)),
        Some(secs) => Err(RewriteError::InvalidConfig(format!(
            "{label} duration must be between {min} and {max} seconds (got {secs})."
        ))),
        None => Err(RewriteError::InvalidConfig(format!(
            "{label} needs a duration between {min} and {max} seconds."
        ))),
    }
}

pub fn render_system(template: &Template, duration: Option<u32>) -> String {
    let (dur_text, clock) = match duration {
        Some(secs) => (secs.to_string(), duration_clock(secs)),
        None => ("unspecified".to_string(), "unspecified".to_string()),
    };
    template
        .body
        .replace("{{duration_clock}}", &clock)
        .replace("{{duration}}", &dur_text)
}

pub fn build_user_message(template: &Template, idea: &str, duration: Option<u32>) -> String {
    let mut msg = String::new();
    msg.push_str("Rewrite the rough idea below into the target prompt.\n\n");
    msg.push_str("Target: ");
    msg.push_str(&template.title);
    msg.push('\n');
    if let Some(secs) = duration {
        msg.push_str(&format!("Duration: {secs} seconds\n"));
    }
    msg.push_str("\nRough idea:\n");
    msg.push_str(idea.trim());
    msg.push('\n');
    msg
}

pub fn validate_idea(idea: &str) -> Result<String, RewriteError> {
    let idea = idea.trim();
    if idea.is_empty() {
        return Err(RewriteError::InvalidConfig(
            "Write an idea before rewriting.".into(),
        ));
    }
    if idea.chars().count() > 8_000 {
        return Err(RewriteError::InvalidConfig(
            "The idea is longer than 8,000 characters.".into(),
        ));
    }
    Ok(idea.to_string())
}

fn split_frontmatter<'a>(filename: &str, source: &'a str) -> Result<(BTreeMap<String, String>, &'a str), RewriteError> {
    let rest = source
        .strip_prefix("---\n")
        .or_else(|| source.strip_prefix("---\r\n"))
        .ok_or_else(|| {
            RewriteError::Template(format!(
                "{filename}: the file must start with a --- frontmatter block."
            ))
        })?;
    let (header, body) = if let Some(idx) = rest.find("\n---\n") {
        (&rest[..idx], &rest[idx + 5..])
    } else if let Some(idx) = rest.find("\n---\r\n") {
        (&rest[..idx], &rest[idx + 6..])
    } else {
        return Err(RewriteError::Template(format!(
            "{filename}: the frontmatter block is missing its closing ---."
        )));
    };
    let mut map = BTreeMap::new();
    for (line_no, line) in header.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(RewriteError::Template(format!(
                "{filename}: frontmatter line {} needs a key and a value.",
                line_no + 1
            )));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(RewriteError::Template(format!(
                "{filename}: frontmatter has an empty key."
            )));
        }
        map.insert(key.to_string(), value.trim().to_string());
    }
    Ok((map, body))
}

fn required(meta: &BTreeMap<String, String>, key: &str, filename: &str) -> Result<String, RewriteError> {
    meta.get(key)
        .filter(|value| !value.is_empty())
        .cloned()
        .ok_or_else(|| RewriteError::Template(format!("{filename}: frontmatter needs `{key}`.")))
}

fn optional_usize(
    meta: &BTreeMap<String, String>,
    key: &str,
    filename: &str,
) -> Result<Option<usize>, RewriteError> {
    let Some(value) = meta.get(key) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    value.parse::<usize>().map(Some).map_err(|_| {
        RewriteError::Template(format!("{filename}: `{key}` must be a whole number."))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_reject_path_traversal() {
        assert!(valid_template_id("wan22-t2v"));
        assert!(valid_template_id("seedance-2.0"));
        assert!(!valid_template_id("../wan"));
        assert!(!valid_template_id("wan/x"));
        assert!(!valid_template_id(".."));
        assert!(!valid_template_id(""));
        assert!(!valid_template_id(".hidden"));
    }

    #[test]
    fn duration_placeholder_does_not_eat_the_clock() {
        let raw = bundled_by_id("minimax-h3").unwrap();
        let template = parse_template("minimax-h3.md", raw).unwrap();
        let system = render_system(&template, Some(6));
        assert!(system.contains("6 seconds") || system.contains("exactly 6"));
        assert!(system.contains("00:06.000"));
        assert!(!system.contains("{{duration"));
        assert!(system.contains("integrated_multimodal_description"));
        assert!(system.contains("overall_soundscape"));
        assert!(system.contains("non_diegetic_music"));
        assert!(system.contains("[Shot"));
    }

    #[test]
    fn duration_bounds() {
        assert!(validate_duration("minimax-h3", Some(3)).is_err());
        assert!(validate_duration("minimax-h3", Some(16)).is_err());
        assert_eq!(validate_duration("minimax-h3", Some(4)).unwrap(), Some(4));
        assert_eq!(validate_duration("minimax-h3", Some(15)).unwrap(), Some(15));
        assert_eq!(validate_duration("wan22-t2v", Some(9)).unwrap(), None);
        assert!(validate_duration("seedance-2.5", None).is_err());
        assert_eq!(validate_duration("seedance-2.5", Some(30)).unwrap(), Some(30));
        assert!(validate_duration("seedance-2.5", Some(31)).is_err());
        assert!(validate_duration("seedance-2.0", Some(16)).is_err());
        assert_eq!(validate_duration("seedance-1.0", Some(5)).unwrap(), Some(5));
    }

    #[test]
    fn bundled_templates_match_the_research_rules() {
        let files = bundled_templates();
        assert!(files.len() >= 8, "expected the eight model templates");
        let mut ids = Vec::new();
        for (name, source) in files {
            let template = parse_template(name, source).unwrap_or_else(|err| panic!("{name}: {err}"));
            assert!(!template.body.is_empty(), "{name} body");
            assert!(
                template.body.to_lowercase().contains("only"),
                "{name} should constrain the model to the prompt itself"
            );
            ids.push(template.id.clone());
            let body = template.body.to_lowercase();
            match template.id.as_str() {
                "wan22-t2v" => {
                    assert!(body.contains("entity"));
                    assert!(body.contains("scene"));
                    assert!(body.contains("motion"));
                    assert!(body.contains("800"));
                    assert!(body.contains("60"));
                    assert!(body.contains("200"));
                    assert!(body.contains("negative"));
                    assert!(body.contains("2.5"));
                    assert_eq!(template.output, OutputKind::WanPair);
                    assert_eq!(template.max_chars, Some(800));
                    assert_eq!(template.negative_max_chars, Some(500));
                }
                "wan22-i2v" => {
                    assert!(body.contains("fixed camera"));
                    assert!(body.contains("100"));
                    assert!(body.contains("motion"));
                    assert_eq!(template.max_words, Some(100));
                    assert_eq!(template.output, OutputKind::WanPair);
                }
                "minimax-h3" => {
                    assert!(body.contains("integrated_multimodal_description"));
                    assert!(body.contains("overall_soundscape"));
                    assert!(body.contains("non_diegetic_music"));
                    assert!(body.contains("[shot"));
                    assert!(body.contains("7000"));
                    assert!(body.contains("picture 1"));
                    assert!(body.contains("hailuo"));
                    assert_eq!(template.max_chars, Some(7000));
                }
                "seedance-2.0" => {
                    assert!(body.contains("shot 1"));
                    assert!(body.contains("timestamp"));
                    assert!(body.contains("one camera"));
                    assert!(body.contains("@image"));
                    assert!(body.contains("subtitle"));
                    assert!(body.contains("watermark"));
                    assert_eq!(template.max_words, Some(1000));
                }
                "seedance-2.5" => {
                    assert!(body.contains("asset map"));
                    assert!(body.contains("summary"));
                    assert!(body.contains("timestamp"));
                    assert!(body.contains("constants"));
                    assert!(body.contains("no subtitles"));
                    assert!(body.contains("no bgm"));
                    assert!(body.contains("30"));
                    assert_eq!(template.max_words, Some(1000));
                }
                "seedance-1.0" => {
                    assert!(body.contains("subject"));
                    assert!(body.contains("action"));
                    assert!(body.contains("camera/scene switch"));
                    assert!(body.contains("--duration"));
                    assert!(body.contains("1000"));
                    assert_eq!(template.max_words, Some(1000));
                }
                "seedream-generate" => {
                    assert!(body.contains("subject"));
                    assert!(body.contains("action"));
                    assert!(body.contains("environment"));
                    assert!(body.contains("double quote"));
                    assert!(body.contains("600"));
                    assert!(body.contains("300"));
                    assert_eq!(template.max_words, Some(600));
                }
                "seedream-edit" => {
                    assert!(body.contains("unchanged"));
                    assert!(body.contains("double quote"));
                    assert!(body.contains("999"));
                    assert_eq!(template.max_words, Some(600));
                }
                other => panic!("unexpected template {other}"),
            }
        }
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 8);
    }

    #[test]
    fn user_message_carries_the_idea_and_duration() {
        let template = parse_template("wan22-t2v.md", bundled_by_id("wan22-t2v").unwrap()).unwrap();
        let msg = build_user_message(&template, "  a cat on a sill  ", None);
        assert!(msg.contains("a cat on a sill"));
        assert!(msg.contains("Wan 2.2"));
        assert!(!msg.contains("  a cat"));
        assert!(validate_idea("   ").is_err());
    }
}
