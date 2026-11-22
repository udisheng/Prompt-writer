use crate::error::RewriteError;
use include_dir::{Dir, include_dir};
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
        return Err(RewriteError::Template(format!(
            "Unknown template id `{id}`."
        )));
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
            )));
        }
    };
    let max_chars_kind = match meta.get("max_chars_kind").map(String::as_str) {
        None => LimitKind::Model,
        Some("model") => LimitKind::Model,
        Some("suggested") => LimitKind::Suggested,
        Some(other) => {
            return Err(RewriteError::Template(format!(
                "{filename}: max_chars_kind `{other}` must be model or suggested."
            )));
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
        "seedance-2.5" => Some(("Seedance 2.5", 4, 30)),
        id if id.starts_with("seedance-") => Some(("Seedance", 4, 15)),
        _ => None,
    }
}

pub fn validate_duration(
    template_id: &str,
    duration: Option<u32>,
) -> Result<Option<u32>, RewriteError> {
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

fn split_frontmatter<'a>(
    filename: &str,
    source: &'a str,
) -> Result<(BTreeMap<String, String>, &'a str), RewriteError> {
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

fn required(
    meta: &BTreeMap<String, String>,
    key: &str,
    filename: &str,
) -> Result<String, RewriteError> {
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
    value
        .parse::<usize>()
        .map(Some)
        .map_err(|_| RewriteError::Template(format!("{filename}: `{key}` must be a whole number.")))
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
        let template = parse_template(
            "clock-sample.md",
            "---\nid: clock-sample\nfamily: sample\nvariant: sample\ntitle: Clock sample\noutput: text\n---\n\nThe clip is exactly {{duration}} seconds and ends at {{duration_clock}}.\n",
        )
        .unwrap();
        let system = render_system(&template, Some(6));
        assert!(system.contains("exactly 6 seconds"));
        assert!(system.contains("00:06.000"));
        assert!(!system.contains("{{duration"));
        let seedance =
            parse_template("seedance-1.0.md", bundled_by_id("seedance-1.0").unwrap()).unwrap();
        let rendered = render_system(&seedance, Some(6));
        assert!(rendered.contains("--duration 6") || rendered.contains("6 seconds"));
        assert!(!rendered.contains("{{duration"));
    }

    #[test]
    fn duration_bounds() {
        assert_eq!(
            validate_duration("minimax-h3-image-t2i", Some(9)).unwrap(),
            None
        );
        assert_eq!(
            validate_duration("minimax-h3-image-edit", None).unwrap(),
            None
        );
        assert_eq!(validate_duration("wan22-t2v", Some(9)).unwrap(), None);
        assert!(validate_duration("seedance-2.5", None).is_err());
        assert_eq!(
            validate_duration("seedance-2.5", Some(30)).unwrap(),
            Some(30)
        );
        assert!(validate_duration("seedance-2.5", Some(31)).is_err());
        assert!(validate_duration("seedance-2.0", Some(16)).is_err());
        assert_eq!(validate_duration("seedance-1.0", Some(5)).unwrap(), Some(5));
    }

    #[test]
    fn bundled_templates_match_the_research_rules() {
        let files = bundled_templates();
        assert!(files.len() >= 9, "expected the nine model templates");
        let mut ids = Vec::new();
        for (name, source) in files {
            let template =
                parse_template(name, source).unwrap_or_else(|err| panic!("{name}: {err}"));
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
                    assert!(body.contains("80 to 120"));
                    assert!(body.contains("negative"));
                    assert!(body.contains("2.5"));
                    assert!(body.contains("fixed camera"));
                    assert!(body.contains("golden hour"));
                    assert!(body.contains("shallow depth of field"));
                    assert!(body.contains("cfg=1"));
                    assert_eq!(template.output, OutputKind::WanPair);
                    assert_eq!(template.max_chars, Some(800));
                    assert_eq!(template.negative_max_chars, Some(500));
                }
                "wan22-i2v" => {
                    assert!(body.contains("fixed camera"));
                    assert!(body.contains("100"));
                    assert!(body.contains("motion"));
                    assert!(body.contains("nobody else"));
                    assert!(body.contains("cfg=1"));
                    assert!(body.contains("800"));
                    assert_eq!(template.max_words, Some(100));
                    assert_eq!(template.max_chars, Some(800));
                    assert_eq!(template.output, OutputKind::WanPair);
                }
                "minimax-h3-image-edit" => {
                    assert!(body.contains("<picture 1>"));
                    assert!(body.contains("<subject 1>"));
                    assert!(body.contains("partially preserved"));
                    assert!(body.contains("do not slim, enlarge, beautify, or reshape"));
                    assert!(body.contains("loosely fitted"));
                    assert!(body.contains("ordinary cloth wrinkles"));
                    assert!(body.contains("beauty-filter smoothing"));
                    assert!(body.contains("the image contains no text, letters or numbers."));
                    assert!(body.contains("1,500"));
                    assert!(body.contains("stylized"));
                    assert!(body.contains("no negative_prompt"));
                    assert!(body.contains("every action verb"));
                    assert!(body.contains("true-to-life colours"));
                    assert!(body.contains("do not add film grain"));
                    assert!(body.contains("do not re-caption"));
                    assert!(body.contains("do not open the prompt with the action"));
                    assert!(body.contains("never write slender, petite, slim, skinny, or curvy"));
                    assert!(
                        body.contains(
                            "if the idea does not name a lens or aperture, do not add one"
                        )
                    );
                    assert!(body.contains("only a format sample"));
                    assert!(body.contains("every picture the idea names"));
                    assert!(!body.contains("including `85mm`"));
                    assert!(!body.contains("integrated_multimodal_description"));
                    assert_eq!(template.max_chars, Some(1500));
                    assert_eq!(template.max_chars_kind, LimitKind::Suggested);
                    assert_eq!(template.output, OutputKind::Text);
                    assert!(template.negative_max_chars.is_none());
                }
                "minimax-h3-image-t2i" => {
                    assert!(body.contains("no reference images"));
                    assert!(body.contains("do not invent"));
                    assert!(body.contains("flowing paragraphs"));
                    assert!(body.contains("2 rows and 3 columns"));
                    assert!(body.contains("beauty-filter smoothing"));
                    assert!(body.contains("the image contains no text, letters or numbers."));
                    assert!(body.contains("1,500"));
                    assert!(body.contains("stylized"));
                    assert!(body.contains("no negative_prompt"));
                    assert!(body.contains("every action verb"));
                    assert!(body.contains("true-to-life colours"));
                    assert!(body.contains("do not add film grain"));
                    assert!(body.contains("do not invent a full person"));
                    assert!(
                        body.contains(
                            "body position, where the hands are, and where the gaze goes"
                        )
                    );
                    assert!(body.contains("never write slender, petite, slim, skinny, or curvy"));
                    assert!(
                        body.contains(
                            "if the idea does not name a lens or aperture, do not add one"
                        )
                    );
                    assert!(body.contains("only a format sample"));
                    assert!(!body.contains("including `85mm`"));
                    assert_eq!(template.max_chars, Some(1500));
                    assert_eq!(template.max_chars_kind, LimitKind::Suggested);
                    assert_eq!(template.output, OutputKind::Text);
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
                    // field notes: labelled lines, one pose each, references bind by number, POV, era
                    for label in [
                        "action:",
                        "subjects:",
                        "outfits:",
                        "background:",
                        "lighting:",
                        "camera:",
                        "style:",
                    ] {
                        assert!(
                            body.contains(label),
                            "seedream-generate is missing the {label} line"
                        );
                    }
                    assert!(body.contains("one pose and one main action per person"));
                    assert!(body.contains("both hands"));
                    assert!(body.contains("do not re-describe the face"));
                    assert!(body.contains("first-person point of view"));
                    assert!(body.contains("do not name the viewer"));
                    assert!(body.contains("no modern clothes"));
                    assert!(body.contains("no text, captions or watermarks"));
                }
                "seedream-edit" => {
                    assert!(body.contains("unchanged"));
                    assert!(body.contains("double quote"));
                    assert!(body.contains("999"));
                    assert_eq!(template.max_words, Some(600));
                    assert!(body.contains("face and identity unchanged"));
                    assert!(body.contains("one thing about a person"));
                }
                other => panic!("unexpected template {other}"),
            }
        }
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 9);
        assert!(!ids.iter().any(|id| id == "minimax-h3"));
    }

    #[test]
    fn h3_image_edit_loads_and_protected_terms_survive_postprocessing() {
        use crate::parse::{measure, parse_output};
        use crate::protect::{
            corrective_message, missing_terms, protection_for, run_keyword_line,
            should_retry_protection, strip_inline_braces, system_addon,
        };

        assert!(bundled_by_id("minimax-h3").is_err());
        let template = parse_template(
            "minimax-h3-image-edit.md",
            bundled_by_id("minimax-h3-image-edit").unwrap(),
        )
        .unwrap();
        let t2i = parse_template(
            "minimax-h3-image-t2i.md",
            bundled_by_id("minimax-h3-image-t2i").unwrap(),
        )
        .unwrap();
        assert_eq!(template.family, "minimax-h3-image");
        assert_eq!(template.variant, "edit");
        assert_eq!(t2i.variant, "t2i");
        assert_eq!(t2i.title, "MiniMax H3-Image · Text to image");

        const GOLDEN: &str = "The woman in <Picture 1> is <Subject 1>. One complete outfit from <Picture 2> is <Subject 2>. The bedroom in <Picture 3> is <Subject 3>. The target image is a mid close-up, waist-up, eye-level photograph of <Subject 1> sitting on the edge of the bed in <Subject 3>, wearing <Subject 2>, torso upright, raising both hands in a peace sign, gaze toward the lens, and a faint smile. <Subject 1> is partially preserved; identity, face, hair, accessories, build, proportions, silhouette, shoulder width, and scale stay as in <Picture 1>, while her pose, clothing, and setting change. Do not slim, enlarge, beautify, or reshape her figure. <Subject 2> is partially preserved; cut, fabric, and colours stay, worn as a loosely fitted top with a very short skirt and ordinary cloth wrinkles rather than a stretched, tight fit. <Subject 3> is partially preserved; the room and the bed stay, framed as this waist-up mid close-up. Shot on an 85mm lens at f/1.8 in soft window light, with true-to-life colours, natural skin texture with visible pores, and realistic depth of field. Avoid beauty-filter smoothing, plastic or airbrushed skin, oversharpening, and HDR halos. The image contains no text, letters or numbers.";
        assert!(
            template.body.contains(GOLDEN),
            "edit template is missing the golden example"
        );
        assert!(
            GOLDEN.chars().count() <= 1500,
            "golden example is {} characters",
            GOLDEN.chars().count()
        );

        let idea = "A portrait of {{Mika}} from <Picture 1>, trigger sks_mika, peace sign.";
        let saved = vec!["sks_mika".to_string()];
        let protection = protection_for(idea, &saved);
        assert!(protection.inline.iter().any(|term| term == "Mika"));
        assert!(protection.required.iter().any(|term| term == "sks_mika"));
        assert!(protection.required.iter().any(|term| term == "Mika"));
        let addon = system_addon(&protection.required);
        assert!(addon.contains("sks_mika"));
        assert!(addon.contains("Mika"));
        let user = build_user_message(&template, idea, None);
        assert!(user.contains("MiniMax H3-Image · Edit"));
        assert!(user.contains("sks_mika"));
        assert!(!user.contains("Duration:"));

        let reply = "```\nThe woman in <Picture 1> is <Subject 1>. {{Mika}} holds a peace sign. sks_mika stays.\n```";
        let parsed = parse_output(template.output, reply);
        let stripped = strip_inline_braces(&parsed.output, &protection.inline);
        assert!(stripped.contains("Mika"));
        assert!(!stripped.contains("{{"));
        assert!(stripped.contains("sks_mika"));
        assert!(missing_terms(&stripped, &protection.required).is_empty());

        let dropped = "The woman in <Picture 1> is <Subject 1>.";
        let missing = missing_terms(dropped, &protection.required);
        assert_eq!(missing.len(), 2);
        assert!(should_retry_protection(0, &missing));
        assert!(!should_retry_protection(1, &missing));
        let corrective = corrective_message(&missing);
        assert!(corrective.contains("sks_mika"));
        assert!(corrective.contains("Mika"));

        let parsed_long = parse_output(template.output, &format!("```\n{}\n```", "x".repeat(1600)));
        assert!(
            parsed_long.negative.is_none(),
            "a text target has no negative field"
        );
        let measured = measure(&template, parsed_long);
        assert!(measured.over_limit);
        assert!(measured.negative.is_none());
        assert!(
            measured
                .warnings
                .iter()
                .any(|warning| warning.contains("suggested length"))
        );

        let shown = run_keyword_line(&saved, "/show key");
        assert!(shown.handled);
        assert!(!shown.changed);
        assert!(shown.message.contains("sks_mika"));
        let added = run_keyword_line(&saved, "/add key peace sign");
        assert!(added.changed);
        assert!(added.keywords.iter().any(|keyword| keyword == "peace sign"));
        assert!(added.keywords.iter().any(|keyword| keyword == "sks_mika"));
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

    #[test]
    fn h3_image_fixture_outputs_keep_actions_and_skip_default_film() {
        use crate::actions::{
            action_addon, action_requirements, missing_action_terms, missing_edit_labels,
        };

        let edit_template = parse_template(
            "minimax-h3-image-edit.md",
            bundled_by_id("minimax-h3-image-edit").unwrap(),
        )
        .unwrap();
        let t2i_template = parse_template(
            "minimax-h3-image-t2i.md",
            bundled_by_id("minimax-h3-image-t2i").unwrap(),
        )
        .unwrap();

        // Hand-written outputs. No provider key was available in this environment,
        // so these follow the template instead of a live model reply.
        let fixtures = [
            (
                "golden edit",
                true,
                "A hyper-realistic middle shot fashion cosplay photograph of a slender lady from <Picture 1> with naturally small chest. Her facial features and accessories remain the same as <Picture 1>. Peace sign, wearing one of the outfits from <Picture 2>, background is <Picture 3>, very short skirt, sitting on the bed, faint smile. 85mm f/1.8, middle close up, masterpiece, best quality, photorealistic.",
                "The woman in <Picture 1> is <Subject 1>. One complete outfit from <Picture 2> is <Subject 2>. The bedroom in <Picture 3> is <Subject 3>. The target image is a mid close-up, waist-up, eye-level photograph of <Subject 1> sitting on the edge of the bed in <Subject 3>, wearing <Subject 2>, torso upright, raising both hands in a peace sign, gaze toward the lens, and a faint smile. <Subject 1> is partially preserved; identity, face, hair, accessories, build, proportions, silhouette, shoulder width, and scale stay as in <Picture 1>, while her pose, clothing, and setting change. Do not slim, enlarge, beautify, or reshape her figure. <Subject 2> is partially preserved; cut, fabric, and colours stay, worn as a loosely fitted top with a very short skirt and ordinary cloth wrinkles rather than a stretched, tight fit. <Subject 3> is partially preserved; the room and the bed stay, framed as this waist-up mid close-up. Shot on an 85mm lens at f/1.8 in soft window light, with true-to-life colours, natural skin texture with visible pores, and realistic depth of field. Avoid beauty-filter smoothing, plastic or airbrushed skin, oversharpening, and HDR halos. The image contains no text, letters or numbers.",
            ),
            (
                "walking with coffee",
                true,
                "The woman from <Picture 1> walking toward the camera holding coffee, wearing the coat from <Picture 2>, on the street in <Picture 3>.",
                "The woman in <Picture 1> is <Subject 1>. The coat in <Picture 2> is <Subject 2>. The street in <Picture 3> is <Subject 3>. The target image is an eye-level photograph of <Subject 1> walking toward the camera on <Subject 3>, wearing <Subject 2>, holding coffee in her right hand at her side, left arm swinging, and her gaze on the lens. <Subject 1> is partially preserved; identity, face, hair, accessories, build, proportions, silhouette, shoulder width, and scale stay as in <Picture 1>, while her pose, clothing, and setting change. Do not slim, enlarge, beautify, or reshape her figure. <Subject 2> is partially preserved; the coat's cut, fabric, and colours stay, worn loosely with ordinary cloth wrinkles rather than a stretched, tight fit. <Subject 3> is partially preserved; the street stays behind her. Natural daylight, true-to-life colours, natural skin texture with visible pores, and realistic depth of field. Avoid beauty-filter smoothing, plastic or airbrushed skin, oversharpening, and HDR halos. The image contains no text, letters or numbers.",
            ),
            (
                "leaning on a railing",
                true,
                "The man from <Picture 1> leaning on a railing looking over his shoulder, jacket from <Picture 2>, rooftop from <Picture 3>.",
                "The man in <Picture 1> is <Subject 1>. The jacket in <Picture 2> is <Subject 2>. The rooftop in <Picture 3> is <Subject 3>. The target image is an eye-level photograph of <Subject 1> leaning on a railing on <Subject 3>, wearing <Subject 2>, one forearm resting on the rail, torso turned away, looking over his shoulder, and his gaze back toward the lens. <Subject 1> is partially preserved; identity, face, hair, accessories, build, proportions, silhouette, shoulder width, and scale stay as in <Picture 1>, while his pose, clothing, and setting change. Do not slim, enlarge, beautify, or reshape his figure. <Subject 2> is partially preserved; the jacket's cut, fabric, and colours stay, worn loosely with ordinary cloth wrinkles rather than a stretched, tight fit. <Subject 3> is partially preserved; the rooftop stays. Natural daylight, true-to-life colours, natural skin texture with visible pores, and realistic depth of field. Avoid beauty-filter smoothing, plastic or airbrushed skin, oversharpening, and HDR halos. The image contains no text, letters or numbers.",
            ),
            (
                "hand turning a bottle",
                false,
                "Product close-up of a hand turning a bottle on a wooden table, soft daylight.",
                "A close-up photograph of a hand turning a bottle on a wooden table. The fingers wrap the bottle and the wrist rotates it a quarter turn toward the viewer. Soft daylight, true-to-life colours, natural skin texture with visible pores, and realistic depth of field. Avoid beauty-filter smoothing, plastic or airbrushed skin, oversharpening, and HDR halos. The image contains no text, letters or numbers.",
            ),
        ];

        for (name, edit, idea, expected) in fixtures {
            let template = if edit { &edit_template } else { &t2i_template };
            let requirements = action_requirements(idea, edit);
            assert!(
                !requirements.terms.is_empty(),
                "{name} produced no action terms"
            );
            let system = format!(
                "{}{}",
                render_system(template, None),
                action_addon(&requirements)
            );
            assert!(system.contains("Actions to keep"), "{name} addon");
            assert!(
                system.to_lowercase().contains("every action verb"),
                "{name} template rule"
            );
            for term in &requirements.terms {
                assert!(
                    system
                        .to_ascii_lowercase()
                        .contains(&term.to_ascii_lowercase()),
                    "{name} system prompt is missing the action term {term}"
                );
            }
            let missing = missing_action_terms(expected, &requirements);
            assert!(
                missing.is_empty(),
                "{name} expected output dropped {missing:?}; terms were {:?}",
                requirements.terms
            );
            let lower = expected.to_ascii_lowercase();
            for banned in [
                "film grain",
                "35mm",
                "muted colours",
                "muted colors",
                "vintage",
            ] {
                assert!(!lower.contains(banned), "{name} contains {banned}");
            }
            if edit {
                let labels = missing_edit_labels(idea, expected);
                assert!(labels.is_empty(), "{name} labels: {labels:?}");
            }
        }

        let dropped = "The woman in <Picture 1> is <Subject 1>. The target image is a portrait of <Subject 1> in the bedroom. <Subject 1> is partially preserved; identity stays, while her pose changes.";
        let requirements = action_requirements(fixtures[0].2, true);
        let missing = missing_action_terms(dropped, &requirements);
        assert!(
            missing
                .iter()
                .any(|term| term.eq_ignore_ascii_case("sitting"))
        );
        assert!(
            missing
                .iter()
                .any(|term| term.eq_ignore_ascii_case("peace sign"))
        );
    }
}
