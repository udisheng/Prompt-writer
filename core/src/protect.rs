use serde_json::Value;

/// Terms that must survive a rewrite: saved keywords that occur in the idea, plus `{{inline}}` terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protection {
    pub inline: Vec<String>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordCommandOutcome {
    pub handled: bool,
    pub changed: bool,
    pub keywords: Vec<String>,
    pub message: String,
}

pub fn parse_keyword_document(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if (trimmed.starts_with('[') || trimmed.starts_with('{'))
        && let Some(from_json) = keywords_from_json(trimmed)
    {
        return dedupe(from_json);
    }
    dedupe(keywords_from_lines(text))
}

pub fn format_keyword_text(keywords: &[String]) -> String {
    if keywords.is_empty() {
        return String::new();
    }
    let mut text = keywords.join("\n");
    text.push('\n');
    text
}

pub fn inline_terms(idea: &str) -> Vec<String> {
    let chars: Vec<char> = idea.chars().collect();
    let mut terms = Vec::new();
    let mut index = 0;
    while index + 1 < chars.len() {
        if chars[index] == '{'
            && chars[index + 1] == '{'
            && let Some(close) = find_close(&chars, index + 2)
        {
            let inner: String = chars[index + 2..close].iter().collect();
            let term = inner.trim();
            if !term.is_empty() {
                push_unique(&mut terms, term.to_string());
            }
            index = close + 2;
            continue;
        }
        index += 1;
    }
    terms
}

pub fn protection_for(idea: &str, saved: &[String]) -> Protection {
    let inline = inline_terms(idea);
    let mut required = Vec::new();
    for term in saved {
        let term = term.trim();
        if term.is_empty() {
            continue;
        }
        if idea.contains(term) {
            push_unique(&mut required, term.to_string());
        }
    }
    for term in &inline {
        push_unique(&mut required, term.clone());
    }
    Protection { inline, required }
}

pub fn missing_terms(output: &str, required: &[String]) -> Vec<String> {
    required
        .iter()
        .filter(|term| !term.is_empty() && !output.contains(term.as_str()))
        .cloned()
        .collect()
}

pub fn should_retry_protection(attempt: u8, missing: &[String]) -> bool {
    attempt == 0 && !missing.is_empty()
}

pub fn system_addon(required: &[String]) -> String {
    if required.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "\n\n## Protected terms\n\
         These strings appear in the rough idea. Copy each one into the rewritten prompt exactly, \
         with the same spelling, case, and punctuation. Do not translate, paraphrase, pluralize, \
         respell, or drop them. If the idea wrapped a term in double braces, write that term \
         without the braces.\n",
    );
    for term in required {
        out.push_str("- ");
        out.push_str(term);
        out.push('\n');
    }
    out
}

pub fn corrective_message(missing: &[String]) -> String {
    let mut out = String::from(
        "The rewrite dropped protected terms. Put every term below back into the prompt exactly, \
         same spelling, case, and punctuation, and do not wrap them in braces. Keep the rest of \
         the prompt and return the same output format as before.\n\nMissing terms:\n",
    );
    for term in missing {
        out.push_str("- ");
        out.push_str(term);
        out.push('\n');
    }
    out
}

pub fn missing_warning(missing: &[String]) -> String {
    format!(
        "Protected terms missing from the rewrite: {}. They were in the idea and have to stay verbatim.",
        missing.join(", ")
    )
}

/// Replace `{{term}}` with `term` when `term` is an inline protected term.
pub fn strip_inline_braces(text: &str, inline: &[String]) -> String {
    if inline.is_empty() || !text.contains("{{") {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '{'
            && index + 1 < chars.len()
            && chars[index + 1] == '{'
            && let Some(close) = find_close(&chars, index + 2)
        {
            let inner: String = chars[index + 2..close].iter().collect();
            let trimmed = inner.trim();
            if inline.iter().any(|term| term == trimmed) {
                out.push_str(trimmed);
                index = close + 2;
                continue;
            }
        }
        out.push(chars[index]);
        index += 1;
    }
    out
}

pub fn run_keyword_line(keywords: &[String], line: &str) -> KeywordCommandOutcome {
    let Some(command) = classify(line) else {
        return KeywordCommandOutcome {
            handled: false,
            changed: false,
            keywords: keywords.to_vec(),
            message: String::new(),
        };
    };
    apply(keywords, command)
}

enum Command {
    Help,
    Show,
    Add(Vec<String>),
    Remove(Vec<String>),
    Clear,
    ClearConfirm,
    Unknown,
}

fn classify(line: &str) -> Option<Command> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix('/')?;
    let rest = rest.trim_start();
    if rest.is_empty() {
        return Some(Command::Unknown);
    }
    let (verb, tail) = split_word(rest);
    let verb = verb.to_ascii_lowercase();
    match verb.as_str() {
        "help" => Some(Command::Help),
        "show" => match key_payload(tail) {
            Some(_) => Some(Command::Show),
            None => Some(Command::Unknown),
        },
        "add" => match key_payload(tail) {
            Some(payload) => Some(Command::Add(split_phrases(&payload))),
            None => Some(Command::Unknown),
        },
        "remove" => match key_payload(tail) {
            Some(payload) => Some(Command::Remove(split_phrases(&payload))),
            None => Some(Command::Unknown),
        },
        "clear" => match key_payload(tail) {
            Some(payload) if payload.is_empty() => Some(Command::Clear),
            Some(payload) if payload.eq_ignore_ascii_case("confirm") => Some(Command::ClearConfirm),
            Some(_) => Some(Command::Clear),
            None => Some(Command::Unknown),
        },
        _ => Some(Command::Unknown),
    }
}

fn apply(keywords: &[String], command: Command) -> KeywordCommandOutcome {
    let mut next = keywords.to_vec();
    let (changed, message) = match command {
        Command::Help => (false, help_text()),
        Command::Show => (false, numbered(&next)),
        Command::Unknown => (
            false,
            "Unknown command. Type /help to see /show key, /add key, /remove key, and /clear key."
                .into(),
        ),
        Command::Clear => (
            false,
            "Nothing was deleted. Type /clear key confirm to remove every protected keyword."
                .into(),
        ),
        Command::ClearConfirm => {
            if next.is_empty() {
                (false, "The protected keyword list is already empty.".into())
            } else {
                let count = next.len();
                next.clear();
                (true, format!("Cleared {count} protected keywords."))
            }
        }
        Command::Add(phrases) => {
            if phrases.is_empty() {
                (
                    false,
                    "Name a word or phrase after /add key. Separate several with commas.".into(),
                )
            } else {
                let mut added = Vec::new();
                let mut already = Vec::new();
                for phrase in phrases {
                    if next.iter().any(|keyword| keyword == &phrase) {
                        push_unique(&mut already, phrase);
                    } else {
                        next.push(phrase.clone());
                        added.push(phrase);
                    }
                }
                let mut message = String::new();
                if !added.is_empty() {
                    message.push_str("Added ");
                    message.push_str(&added.join(", "));
                    message.push('.');
                }
                if !already.is_empty() {
                    if !message.is_empty() {
                        message.push(' ');
                    }
                    message.push_str("Already in the list: ");
                    message.push_str(&already.join(", "));
                    message.push('.');
                }
                message.push('\n');
                message.push_str(&numbered(&next));
                (!added.is_empty(), message)
            }
        }
        Command::Remove(selectors) => {
            if selectors.is_empty() {
                (
                    false,
                    "Name a keyword, a phrase, or a number from /show key.".into(),
                )
            } else {
                let (removed, missing) = remove_selectors(&mut next, &selectors);
                if removed.is_empty() && missing.is_empty() {
                    (
                        false,
                        "Name a keyword, a phrase, or a number from /show key.".into(),
                    )
                } else {
                    let mut message = String::new();
                    if !removed.is_empty() {
                        message.push_str("Removed ");
                        message.push_str(&removed.join(", "));
                        message.push('.');
                    }
                    if !missing.is_empty() {
                        if !message.is_empty() {
                            message.push(' ');
                        }
                        message.push_str("Not in the list: ");
                        message.push_str(&missing.join(", "));
                        message.push_str(". Use /show key to see the numbers.");
                    }
                    if !removed.is_empty() {
                        message.push('\n');
                        message.push_str(&numbered(&next));
                    }
                    (!removed.is_empty(), message)
                }
            }
        }
    };
    KeywordCommandOutcome {
        handled: true,
        changed,
        keywords: next,
        message,
    }
}

fn remove_selectors(
    keywords: &mut Vec<String>,
    selectors: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut drop = vec![false; keywords.len()];
    let mut removed = Vec::new();
    let mut missing = Vec::new();
    for selector in selectors {
        if let Some(pos) = keywords.iter().position(|keyword| keyword == selector) {
            mark(&mut drop, &mut removed, keywords, pos);
            continue;
        }
        if let Some(index) = selector_index(selector)
            && index >= 1
            && index <= keywords.len()
        {
            mark(&mut drop, &mut removed, keywords, index - 1);
            continue;
        }
        let matches: Vec<usize> = keywords
            .iter()
            .enumerate()
            .filter(|(_, keyword)| keyword.eq_ignore_ascii_case(selector))
            .map(|(index, _)| index)
            .collect();
        if matches.len() == 1 {
            mark(&mut drop, &mut removed, keywords, matches[0]);
            continue;
        }
        push_unique(&mut missing, selector.clone());
    }
    let kept = keywords
        .iter()
        .enumerate()
        .filter(|(index, _)| !drop[*index])
        .map(|(_, keyword)| keyword.clone())
        .collect();
    *keywords = kept;
    (removed, missing)
}

fn mark(drop: &mut [bool], removed: &mut Vec<String>, keywords: &[String], pos: usize) {
    if !drop[pos] {
        removed.push(keywords[pos].clone());
        drop[pos] = true;
    }
}

fn selector_index(selector: &str) -> Option<usize> {
    if selector.is_empty() || !selector.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    selector.parse::<usize>().ok()
}

fn help_text() -> String {
    "\
Commands stay in this app and are not sent to the model.
/show key — list protected keywords
/add key <word or phrase> — add one, or several separated by commas
/remove key <word, phrase, or number> — remove a keyword
/clear key — ask first
/clear key confirm — delete every protected keyword
/help — show this list
A keyword in the list is copied verbatim when it appears in an idea. {{double braces}} protect a term for that idea only, and the braces are left out of the rewrite."
        .into()
}

fn numbered(keywords: &[String]) -> String {
    if keywords.is_empty() {
        return "No protected keywords yet. Add one with /add key <word or phrase>.".into();
    }
    let mut out = String::from("Protected keywords:\n");
    for (index, keyword) in keywords.iter().enumerate() {
        out.push_str(&format!("{}. {keyword}\n", index + 1));
    }
    out.trim_end().to_string()
}

fn keywords_from_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn keywords_from_json(text: &str) -> Option<Vec<String>> {
    let value: Value = serde_json::from_str(text).ok()?;
    let items = if let Some(array) = value.as_array() {
        array
    } else {
        value.get("keywords")?.as_array()?
    };
    let mut keywords = Vec::new();
    for item in items {
        if let Some(text) = item.as_str() {
            let text = text.trim();
            if !text.is_empty() {
                keywords.push(text.to_string());
            }
        }
    }
    Some(keywords)
}

fn dedupe(keywords: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for keyword in keywords {
        push_unique(&mut out, keyword);
    }
    out
}

fn push_unique(list: &mut Vec<String>, value: String) {
    if !value.is_empty() && !list.iter().any(|existing| existing == &value) {
        list.push(value);
    }
}

fn split_word(text: &str) -> (&str, &str) {
    let text = text.trim_start();
    match text.find(char::is_whitespace) {
        Some(index) => (&text[..index], text[index..].trim_start()),
        None => (text, ""),
    }
}

fn key_payload(tail: &str) -> Option<String> {
    let (word, rest) = split_word(tail);
    if word.eq_ignore_ascii_case("key") {
        Some(rest.trim().to_string())
    } else {
        None
    }
}

fn split_phrases(payload: &str) -> Vec<String> {
    let mut phrases = Vec::new();
    for part in payload.split(',') {
        let part = part.trim();
        if !part.is_empty() {
            push_unique(&mut phrases, part.to_string());
        }
    }
    phrases
}

fn find_close(chars: &[char], from: usize) -> Option<usize> {
    let mut index = from;
    while index + 1 < chars.len() {
        if chars[index] == '}' && chars[index + 1] == '}' {
            return Some(index);
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braces_extract_the_inner_text_once() {
        let terms = inline_terms("hello {{Mika}} and {{ neon noir }} and {{Mika}}");
        assert_eq!(terms, vec!["Mika".to_string(), "neon noir".to_string()]);
        assert!(inline_terms("unclosed {{Mika").is_empty());
        assert!(inline_terms("{{}} and {{   }}").is_empty());
        assert_eq!(
            inline_terms("a {{b}} c {{d e}}"),
            vec!["b".to_string(), "d e".to_string()]
        );
    }

    #[test]
    fn braces_are_stripped_only_for_protected_terms() {
        let inline = vec!["Mika".to_string(), "neon noir".to_string()];
        assert_eq!(
            strip_inline_braces(
                "Hello {{Mika}}, stay {{ neon noir }}. Keep {{other}}.",
                &inline
            ),
            "Hello Mika, stay neon noir. Keep {{other}}."
        );
        assert_eq!(strip_inline_braces("plain Mika", &inline), "plain Mika");
    }

    #[test]
    fn post_check_requires_exact_terms_that_appear_in_the_idea() {
        let saved = vec!["Mika".into(), "zxc_lora".into(), "unused".into()];
        let protection = protection_for("Mika walks with {{zxc_lora}}", &saved);
        assert_eq!(
            protection.required,
            vec!["Mika".to_string(), "zxc_lora".to_string()]
        );
        assert_eq!(protection.inline, vec!["zxc_lora".to_string()]);
        assert!(
            missing_terms("Mika walks", &protection.required).contains(&"zxc_lora".to_string())
        );
        assert!(missing_terms("Mika walks with zxc_lora", &protection.required).is_empty());
        assert_eq!(
            missing_terms("mika walks with zxc_lora", &protection.required),
            vec!["Mika".to_string()]
        );
        let absent = protection_for("a cat on a sill", &saved);
        assert!(absent.required.is_empty());
        assert!(!should_retry_protection(0, &[]));
        assert!(should_retry_protection(0, &protection.required));
        assert!(!should_retry_protection(1, &protection.required));
    }

    #[test]
    fn addon_and_warning_name_the_terms() {
        assert!(system_addon(&[]).is_empty());
        let addon = system_addon(&["Mika".into(), "zxc_lora".into()]);
        assert!(addon.contains("same spelling, case, and punctuation"));
        assert!(addon.contains("- Mika\n"));
        assert!(addon.contains("- zxc_lora\n"));
        let warning = missing_warning(&["Mika".into(), "neon noir".into()]);
        assert!(warning.contains("Mika, neon noir"));
        let correction = corrective_message(&["Mika".into()]);
        assert!(correction.contains("- Mika\n"));
        assert!(correction.contains("do not wrap them in braces"));
    }

    #[test]
    fn keyword_file_accepts_lines_comments_and_json() {
        assert_eq!(
            parse_keyword_document("# notes\n\nMika\nneon noir\nMika\n"),
            vec!["Mika".to_string(), "neon noir".to_string()]
        );
        assert_eq!(
            parse_keyword_document(r#"["Mika", " neon noir ", "Mika", 4]"#),
            vec!["Mika".to_string(), "neon noir".to_string()]
        );
        assert_eq!(
            parse_keyword_document(r#"{"keywords":["zxc_lora","zxc_lora"]}"#),
            vec!["zxc_lora".to_string()]
        );
        assert_eq!(format_keyword_text(&["Mika".into()]), "Mika\n");
        assert_eq!(format_keyword_text(&[]), "");
    }

    #[test]
    fn slash_commands_add_show_remove_and_clear() {
        let empty = run_keyword_line(&[], "a cat /add key Mika");
        assert!(!empty.handled);

        let help = run_keyword_line(&[], "/HELP");
        assert!(help.handled && !help.changed);
        assert!(help.message.contains("/add key"));
        assert!(help.message.contains("/clear key confirm"));

        let added = run_keyword_line(&[], "/add key Mika, neon noir, zxc_lora, Mika");
        assert!(added.changed);
        assert_eq!(
            added.keywords,
            vec![
                "Mika".to_string(),
                "neon noir".to_string(),
                "zxc_lora".to_string()
            ]
        );
        assert!(added.message.contains("1. Mika"));
        assert!(added.message.contains("3. zxc_lora"));

        let again = run_keyword_line(&added.keywords, "/ADD KEY Mika");
        assert!(!again.changed);
        assert!(again.message.contains("Already in the list: Mika."));
        assert_eq!(again.keywords.len(), 3);

        let shown = run_keyword_line(&added.keywords, "/show key");
        assert!(!shown.changed);
        assert!(shown.message.contains("2. neon noir"));

        let removed = run_keyword_line(&added.keywords, "/remove key neon noir");
        assert!(removed.changed);
        assert_eq!(
            removed.keywords,
            vec!["Mika".to_string(), "zxc_lora".to_string()]
        );

        let by_number = run_keyword_line(&added.keywords, "/remove key 2");
        assert_eq!(
            by_number.keywords,
            vec!["Mika".to_string(), "zxc_lora".to_string()]
        );

        let by_case = run_keyword_line(&added.keywords, "/remove key mika");
        assert_eq!(by_case.keywords[0], "neon noir");

        let missing = run_keyword_line(&added.keywords, "/remove key nobody");
        assert!(!missing.changed);
        assert!(missing.message.contains("Not in the list: nobody"));

        let numbered_keyword = vec!["2".to_string(), "Mika".to_string()];
        let text_wins = run_keyword_line(&numbered_keyword, "/remove key 2");
        assert_eq!(text_wins.keywords, vec!["Mika".to_string()]);

        let cleared_ask = run_keyword_line(&added.keywords, "/clear key");
        assert!(!cleared_ask.changed);
        assert_eq!(cleared_ask.keywords.len(), 3);
        assert!(cleared_ask.message.contains("/clear key confirm"));

        let cleared = run_keyword_line(&added.keywords, "/clear KEY confirm");
        assert!(cleared.changed);
        assert!(cleared.keywords.is_empty());
        assert!(cleared.message.contains("Cleared 3"));

        let unknown = run_keyword_line(&[], "/camera pushes in");
        assert!(unknown.handled && !unknown.changed);
        assert!(unknown.message.contains("Unknown command"));

        let bare = run_keyword_line(&[], "/add key");
        assert!(bare.message.contains("Name a word or phrase"));
        assert!(
            run_keyword_line(&[], "/show")
                .message
                .contains("Unknown command")
        );
    }
}
